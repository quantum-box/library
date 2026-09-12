#!/usr/bin/env bash
# Used by ephemeral Cloud App builders; local developers can install the same
# pinned worker-build version directly. Does not configure cloud credentials.
set -euo pipefail
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
if ! command -v rustup >/dev/null 2>&1; then
  installer="$(mktemp)"
  trap 'rm -f "$installer"' EXIT
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error https://sh.rustup.rs -o "$installer"
  sh "$installer" -y --profile minimal --default-toolchain nightly-2026-06-04 --no-modify-path
fi
rustup toolchain install nightly-2026-06-04 --profile minimal --no-self-update
rustup target add wasm32-unknown-unknown --toolchain nightly-2026-06-04
if ! command -v worker-build >/dev/null 2>&1 || [ "$(worker-build --version)" != '0.8.5' ]; then
  cargo +nightly-2026-06-04 install worker-build --version 0.8.5 --locked
fi
