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

## Cutting a release

1. Bump `version` in `apps/client/package.json`. `src-tauri/tauri.conf.json` reads
   its version from that file, so the bundled app version and the release tag stay
   in sync.
2. Push a `library-v<version>` tag, or run the workflow manually with that tag. The
   workflow refuses to build if the tag and `package.json` disagree.

The four platform builds run one at a time (`max-parallel: 1`). They all rewrite
the release's shared `latest.json`, and tauri-action deletes the old asset before
uploading its replacement, so overlapping legs race on that delete and the loser
fails with a 404. A whole release therefore takes roughly 35 minutes.

The workflow creates the release as a draft and only publishes it once every leg
has uploaded, so a failed build leaves an unpublished draft rather than a broken
feed — `library-v0.1.5` was left that way by exactly the race above.

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
bar under `Library Client ▸ Check for Updates…`; `src-tauri/src/macos_menu.rs`
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
