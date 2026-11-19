#!/usr/bin/env python3
"""Install the pinned verifier and record complete, uncached verification."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
sys.dont_write_bytecode = True
import tempfile
import time
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[2]
LOCK = ROOT / "scripts/verus/toolchain.json"
SOURCE = ROOT / "packages/verified_org_scope/src/lib.rs"


def mutation_workers(cpu_max=Path("/sys/fs/cgroup/cpu.max")):
    """How many verifier processes to run at once.

    `VERUS_JOBS` wins. Otherwise the cgroup CPU quota bounds it, because
    `os.cpu_count()` reports the node's cores inside a container and a
    1-CPU runner Pod then starts one verifier per node core and runs out
    of memory.
    """
    configured = os.environ.get("VERUS_JOBS")
    if configured:
        return max(1, int(configured))
    cpus = os.cpu_count() or 2
    try:
        quota, period = cpu_max.read_text().split()
        if quota != "max":
            cpus = min(cpus, max(1, int(quota) // int(period)))
    except (OSError, ValueError):
        pass
    return cpus
# Registered production kernels: the verified source, its edition, the
# directories whose Rust feeds or calls it (hashed as evidence), the module
# that yields its mutations, and where `--all` records its evidence.
KERNELS = {
    "packages/verified_org_scope/src/lib.rs": {
        "edition": "2021", "mutations": "org_scope_mutations", "output": "org-scope.json",
        "inputs": ["packages/verified_org_scope/src", "apps/api/src/usecase/library_org.rs",
                   "apps/api/src/usecase/create_repo.rs", "apps/api/src/domain/organization.rs",
                   "apps/api/src/handler/library_executor_extractor.rs"],
    },
}


def run(command, **kwargs):
    return subprocess.run(command, text=True, capture_output=True, timeout=300, **kwargs)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def install(lock):
    arch = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "x86"}[platform.machine()]
    system = {"Darwin": "macos", "Linux": "linux"}[platform.system()]
    asset = f"{arch}-{system}"
    cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    destination = cache / "tachyon-verus" / lock["version"] / asset
    binary = destination / f"verus-{asset}" / "verus"
    if not binary.exists():
        destination.mkdir(parents=True, exist_ok=True)
        name = f"verus-{lock['version']}-{asset}.zip"
        url = f"https://github.com/verus-lang/verus/releases/download/release/{lock['version']}/{name}"
        with tempfile.TemporaryDirectory(dir=destination) as temporary:
            archive = Path(temporary) / name
            urllib.request.urlretrieve(url, archive)
            if sha256(archive) != lock["assets"][asset]:
                raise RuntimeError("Verifier archive checksum mismatch")
            with zipfile.ZipFile(archive) as package:
                package.extractall(temporary)
                for entry in package.infolist():
                    mode = entry.external_attr >> 16
                    if mode:
                        (Path(temporary) / entry.filename).chmod(mode)
            (Path(temporary) / f"verus-{asset}").rename(binary.parent)
    result = run(["rustup", "run", lock["rust"], "rustc", "--version"])
    if result.returncode:
        result = run(["rustup", "toolchain", "install", lock["rust"], "--profile", "minimal"])
        if result.returncode:
            raise RuntimeError(result.stderr)
    return binary


def verify(binary, source, edition="2024"):
    result = run([str(binary), "--edition", edition, "--crate-type", "lib", str(source), "--no-cheating",
                  "--triggers-mode", "silent", "--output-json"], cwd=ROOT)
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"Verifier did not produce JSON: {result.stderr}") from error
    summary = report.get("verification-results", {})
    success = (result.returncode == 0 and summary.get("success") is True
               and summary.get("verified", 0) > 0 and summary.get("errors") == 0
               and summary.get("is-verifying-entire-crate") is True)
    return success, report, result.stderr


def verify_kernel(binary, lock, solver, commit, name, output, self_test, mutations):
    kernel = KERNELS[name]
    source = ROOT / name
    edition = kernel["edition"]
    started = time.monotonic()
    success, report, stderr = verify(binary, source, edition)
    evidence = {
        "commit": commit,
        "working_tree_dirty": bool(run(["git", "status", "--porcelain"], cwd=ROOT).stdout.strip()),
        "invocation": sys.argv,
        "source": name,
        "source_sha256": sha256(source),
        "edition": edition,
        "inputs": {
            str(path.relative_to(ROOT)): sha256(path)
            for directory in kernel["inputs"]
            for path in ([ROOT / directory] if directory.endswith(".rs") else sorted((ROOT / directory).rglob("*.rs")))
            if path.exists()
        },
        "toolchain_sha256": sha256(LOCK),
        "verifier_sha256": sha256(binary),
        "solver": solver,
        "report": report,
        "stderr": stderr,
        "self_tests": {},
    }
    if self_test and success:
        with tempfile.TemporaryDirectory() as temporary:
            false_source = Path(temporary) / "false_postcondition.rs"
            false_source.write_text("use vstd::prelude::*;\nverus! { pub fn false_claim() -> (r: bool) ensures r, { false } }\n")
            accepted, false_report, _ = verify(binary, false_source, edition)
            rejected = (not accepted and false_report.get("verification-results", {}).get("errors", 0) > 0)
            evidence["self_tests"]["false_postcondition_rejected"] = rejected
            success = success and rejected
    if mutations and success:
        cases = importlib.import_module(kernel["mutations"]).cases
        original = source.read_text()
        with tempfile.TemporaryDirectory() as temporary:
            paths = []
            for case, mutated in cases(original):
                if mutated == original:
                    raise RuntimeError(f"Mutation {case} does not change {name}")
                mutation = Path(temporary) / f"{case}.rs"
                mutation.write_text(mutated)
                paths.append((case, mutation))
            # Each mutant is an independent verifier process.
            with ThreadPoolExecutor(max_workers=mutation_workers()) as pool:
                outcomes = list(pool.map(lambda item: (item[0], verify(binary, item[1], edition)), paths))
            for case, (accepted, mutation_report, diagnostics) in outcomes:
                rejected = (not accepted and mutation_report.get("verification-results", {}).get("errors", 0) > 0)
                evidence["self_tests"][case] = {"rejected": rejected, "report": mutation_report, "stderr": diagnostics}
                success = success and rejected
    evidence["duration_seconds"] = round(time.monotonic() - started, 3)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps({"success": success, "source": name, "evidence": str(output),
                      "verification": report.get("verification-results"),
                      "self_tests": {case: value if isinstance(value, bool) else value["rejected"]
                                     for case, value in evidence["self_tests"].items()}}))
    return success


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--mutations", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / "target/verus/evidence.json")
    parser.add_argument("--source", default=str(SOURCE.relative_to(ROOT)), choices=sorted(KERNELS))
    parser.add_argument("--all", action="store_true",
                        help="verify every registered kernel; evidence goes to target/verus/<kernel output>")
    parser.add_argument("--expected-commit")
    args = parser.parse_args()
    commit = run(["git", "rev-parse", "HEAD"], cwd=ROOT).stdout.strip()
    if args.expected_commit and commit != args.expected_commit:
        raise RuntimeError("Verification checkout does not match the requested commit")
    lock = json.loads(LOCK.read_text())
    binary = install(lock)
    version = run([str(binary), "--version", "--output-json"])
    if version.returncode or json.loads(version.stdout)["verus"]["commit"] != lock["commit"]:
        raise RuntimeError("Unexpected verifier version")
    solver = run([str(binary.parent / "z3"), "--version"])
    if solver.returncode or not solver.stdout.startswith(lock["solver"]):
        raise RuntimeError("Unexpected solver version")
    targets = ([(name, ROOT / "target/verus" / kernel["output"]) for name, kernel in KERNELS.items()]
               if args.all else [(args.source, args.output)])
    results = [verify_kernel(binary, lock, solver.stdout.strip(), commit, name, output,
                             args.self_test, args.mutations)
               for name, output in targets]
    if not all(results):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
