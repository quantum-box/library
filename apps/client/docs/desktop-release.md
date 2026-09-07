# Desktop release and self-update

The desktop app ships through GitHub Releases on `quantum-box/library` and updates
itself from that same feed via `tauri-plugin-updater`.

## Release feed

`.github/workflows/desktop-release.yml` builds macOS (arm64 + x86_64), Linux, and
Windows bundles and uploads them to a release, together with a `latest.json`
manifest (`includeUpdaterJson: true`). The app polls:

```
https://github.com/quantum-box/library/releases/latest/download/latest.json
```

`releases/latest` resolves to the newest published, non-prerelease release in the
whole repository — so any other component that starts publishing releases here
would hijack the update feed. Keep this repository's releases to the desktop
client, or move the feed to an explicit tag URL.

## Automatic releases after merge

Every push to `main` (including each merged PR) starts Desktop Release. There are
no path filters: API-only and documentation PRs also produce desktop releases.
The workflow runs the client lint, tests, type check, and build before packaging.
It creates a draft first and publishes only after all four platform builds pass
and the updater manifest has every platform, a signature, and a nonempty matching
release asset. Failed builds keep the previous public feed intact.

Version allocation is automatic. `scripts/desktop-release/config.json` fixes the
last pre-automation main commit and version `0.1.7`. Each subsequent first-parent
main commit increments the patch: the first merge ships `0.1.8`, the next `0.1.9`,
and so on. First-parent counting handles merge and squash commits; a rebase merge
with multiple commits can leave gaps, but its final push still releases once.
The mapping is stable across retries and execution order. Do not edit this anchor
or manually mint `library-v*` tags during normal release work.

The workflow creates a child commit of the exact merged source, synchronizes
`apps/client/package.json` and its npm lockfile, records source/version/tag in
`desktop-release.json`, and pushes only `library-v<version>`. All binaries build
from that tag. Main's package version remains a development placeholder; the
release tag, bundled app, and updater manifest carry the definitive version.
This requires no direct writes to protected main, no version-only PRs, and no PAT.
Tauri reads the package version; the Rust crate version is not the app version.

The entire release workflow uses a shared concurrency group with `queue: max`
and `cancel-in-progress: false`. Successive merges wait instead of replacing the
pending run. GitHub supports at most 100 waiting runs in this queue; overflow is
canceled and must be retried. Within a release, platform builds remain serial
because tauri-action updates the same `latest.json` asset. A release takes about
35 minutes plus validation and queue time.

The published manifest uses tag-specific download URLs, so fetching a manifest
just before another release cannot pair an old signature with a newer binary.
An older retried version is published with `make_latest: false` when a newer
stable Library release already exists. After publishing, the workflow downloads
and validates the public version-specific manifest; failure stays visible in
Actions. A successful source merge or PR CI alone is not release completion.

## Retry and verify

- Rerun a failed Desktop Release run, or dispatch it on **main** with the full
  original main commit SHA in `source_sha`. Empty input releases the dispatch's
  main SHA. Branch commits and commits at/before the anchor are rejected.
- Retries reuse the existing tag and draft. If already published, packaging is
  skipped and the public manifest is checked again. Published assets are never
  replaced by a retry. A conflicting tag fails rather than being overwritten.
- Verify all release jobs, the published `library-v<version>` assets and public
  `latest.json`, then Check for Updates in an installed app. App installation and
  the original application error remain separate checks.
- For a major/minor reset or MSI patch-limit rollover (65535), deliberately choose
  a new anchor/base above all previously published versions in a reviewed change.

The automation uses the workflow's short-lived `GITHUB_TOKEN` with `contents:
write` only for release preparation, asset upload, and publication. Repository
settings, permissions and signing-key changes remain governance Terraform work.

## The pinned Windows upgrade code

`bundle.windows.wix.upgradeCode` in `tauri.conf.json` is pinned to
`053a7581-a632-5412-942e-82424a9627c5`, the UUID v5 that Tauri derives in the DNS
namespace from `Library Client.exe.app.x64` — the product name this app shipped
under through 0.1.6.

Windows identifies an MSI product by its upgrade code. Tauri generates one from
`<productName>.exe.app.x64` unless told otherwise, so renaming the product would
silently change it and Windows would install the update *alongside* the old
version instead of over it. The code is pinned to the old derivation so existing
installs keep upgrading. Never change it. `npx tauri inspect wix-upgrade-code`
prints what the current config would otherwise generate.

## Required repository secrets

| Secret | Purpose |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | minisign private key. The app refuses any update whose signature does not match the `plugins.updater.pubkey` baked into `tauri.conf.json`. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passphrase for that key (empty string if generated without one). |

Losing the private key means shipped clients can no longer be updated — they will
reject bundles signed by any replacement key, and every user has to reinstall by
hand. Back it up somewhere durable.

To rotate or regenerate:

```bash
npx tauri signer generate -w ~/.tauri/library-client-updater.key
```

Then put the public key in `plugins.updater.pubkey` and the private key in the
repository secret. Both halves must ship together.

## Client behaviour

`src/lib/appUpdate.ts` and `src/components/AppUpdateNotice.tsx` implement the flow:
a background check five seconds after launch, plus a manual "Check for updates"
entry in the account menu. On macOS the same check also sits in the native menu
bar under `Library ▸ Check for Updates…`; `src-tauri/src/macos_menu.rs`
builds that item and emits `library-check-for-updates` to the front tab, which
`listenForMenuUpdateCheck()` bridges onto the in-app path. When an update exists
the user is shown the version and release notes and chooses whether to install;
installing downloads, applies, and relaunches.

The updater plugin is registered on desktop targets only, so the web build and the
iOS/Android builds never reach it — `isDesktopApp()` gates every call.

## Code signing

macOS bundles are neither signed nor notarized, and Windows bundles are not
codesigned. That does not block self-update — Tauri verifies its own minisign
signature — but first-time installs still trip Gatekeeper and SmartScreen.
