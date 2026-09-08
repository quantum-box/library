# iOS TestFlight Releases

Every push to `main` builds a signed iOS archive and uploads it to TestFlight.
The workflow is [`.github/workflows/ios-testflight.yml`](../../../.github/workflows/ios-testflight.yml);
it can also be run by hand from the Actions tab, optionally with an explicit
build number.

## What the workflow does

1. Generates the Xcode project with `tauri ios init`. The project is not
   committed — `src-tauri/gen/` is ignored — so the App Store build comes out of
   `src-tauri/tauri.conf.json` exactly like a local one does.
2. Builds with `tauri ios build --export-method app-store-connect
   --build-number <run number>`.
3. Uploads the resulting IPA with `xcrun altool --upload-app`.

The app version comes from `apps/client/package.json`, the same version the
desktop release tags. `--build-number` appends the workflow run number, so
`CFBundleVersion` is `<version>.<run>`: unique and increasing across uploads,
which is what App Store Connect requires — it rejects a build number it has
already accepted for a version.

The IPA is also attached to the run as an artifact for 14 days, so a build that
uploaded but failed processing can still be inspected.

## Required setup

The app record must already exist in App Store Connect for the bundle
identifier in `tauri.conf.json`:

```text
com.quantumbox.library.client
```

Create an App Store Connect API key (Users and Access → Integrations → App Store
Connect API) with the **App Manager** role (Admin also works). The role matters:
the same key both provisions the signing certificate and uploads the build, and
a Developer-role key can do neither. Then set three repository secrets:

| Secret | Value |
| --- | --- |
| `APPLE_API_KEY_ID` | The key ID, e.g. `2X9R4HXF34` |
| `APPLE_API_ISSUER_ID` | The issuer ID shown above the keys table |
| `APPLE_API_KEY_P8` | The downloaded `AuthKey_<id>.p8`, base64 encoded |

```bash
base64 -i AuthKey_2X9R4HXF34.p8 | pbcopy
```

```bash
gh secret set APPLE_API_KEY_ID
gh secret set APPLE_API_ISSUER_ID
gh secret set APPLE_API_KEY_P8
```

Apple issues the `.p8` exactly once, so keep a copy outside CI.

Until all three secrets exist the workflow skips on `main` with a warning rather
than failing the branch; a manual run fails, because someone asked for an upload
and did not get one.

The Apple team is `J8429VCGMR`, set as `bundle.iOS.developmentTeam` in
`tauri.conf.json`. `APPLE_DEVELOPMENT_TEAM` overrides it for a different team.

## Signing

Signing is Xcode automatic signing driven by the API key: `tauri ios build`
passes `-allowProvisioningUpdates` with the key, and App Store Connect issues a
cloud-managed distribution certificate and provisioning profile. Nothing has to
be exported from anyone's Keychain, which is the point — a certificate stored as
a CI secret expires, and its private key lives in one person's login keychain.

If the team ever has to pin a specific certificate and profile instead, set
`IOS_CERTIFICATE` (base64 of the `.p12`), `IOS_CERTIFICATE_PASSWORD`, and
`IOS_MOBILE_PROVISION` (base64 of the `.mobileprovision`) as secrets. The
workflow already passes them through, and Tauri switches to manual signing when
it sees them.

## Failure modes worth recognizing

- **`No IPA was produced by the iOS build`** — the archive step failed earlier;
  read the `xcodebuild` output above it rather than this step.
- **`The provided entity includes an attribute with a value that has already
  been used`** — the build number is not unique. Re-run with an explicit larger
  `build_number` input; this happens when the run number is reset or a build
  was uploaded from a laptop.
- **Upload succeeds, build never appears** — App Store Connect processing
  failed, usually on missing export compliance or an invalid icon. The
  rejection arrives by email, not in the workflow log.
- **Signing asks for a device-provisioning profile** — the export method
  defaulted away from `app-store-connect`; check the build step's flags.

## Local reproduction

```bash
cd apps/client
npm ci
npm run tauri -- ios init
npm run tauri -- ios build --export-method app-store-connect --build-number 1
```

Uploading by hand needs the same key in
`~/.appstoreconnect/private_keys/AuthKey_<id>.p8`:

```bash
xcrun altool --upload-app --type ios \
  --file src-tauri/gen/apple/build/arm64/Library.ipa \
  --apiKey "$APPLE_API_KEY_ID" --apiIssuer "$APPLE_API_ISSUER_ID"
```
