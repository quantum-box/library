# Android Firebase App Distribution

Every push to `main` builds a signed APK and distributes it to testers through
Firebase App Distribution. The workflow is
[`.github/workflows/android-firebase-distribution.yml`](../../../.github/workflows/android-firebase-distribution.yml);
it can also be run by hand from the Actions tab, optionally with an explicit
version code, or with `build_only` to produce an APK without distributing it.

It is the Android counterpart of what [`ios-testflight.md`](./ios-testflight.md)
does for iOS, and deliberately mirrors it: same concurrency contract, same
skip-quietly-until-configured credentials gate, same artifact retention. What it
is not is a store release. Google Play needs a developer account this project
does not currently have, and App Distribution needs none — testers install from
a link. If the app ever goes to Play, the workflow changes shape (an AAB, Play
App Signing, the Play Developer API) but the build up to signing does not.

## What the workflow does

1. Generates the Gradle project with `tauri android init`, then copies the
   launcher icons in with `npm run tauri:android:icon`. The project is not
   committed — `src-tauri/gen/` is ignored — so the distributed build comes out
   of `src-tauri/tauri.conf.json` plus `src-tauri/tauri.android.conf.json`
   exactly like a local one does.
2. Builds `--apk` for `aarch64` and `x86_64` with `bundle.android.versionCode`
   set to the workflow run number.
3. Aligns it with `zipalign` and signs it with `apksigner`.
4. Uploads it with `firebase appdistribution:distribute` to the tester groups in
   the `FIREBASE_TESTER_GROUPS` variable, defaulting to `testers`.

An APK rather than an AAB: App Distribution accepts a bundle only for an app
linked to Google Play, and testers here install the binary directly.

`versionName` is the version in `apps/client/package.json`, the same version the
desktop release tags. `versionCode` is the workflow run number on its own.
Android refuses to install a version code lower than the one already on the
device, so it has to increase on every build a tester might take. Tauri's
default is derived from the semver (`major * 1000000 + minor * 1000 + patch`, so
`0.1.39` → `1039`), which repeats itself on every rebuild of the same version —
that is why the workflow sets it explicitly.

Only `aarch64` and `x86_64` are built. arm64 covers every phone a tester will
use and x86_64 keeps emulators and Chromebooks installable; `armv7` would mean a
third full Rust build for devices that predate 2017. Add it to the build step's
`--target` flags if one ever has to be supported.

When pushes arrive faster than a build takes, GitHub keeps only one pending run
per concurrency group, so the newest main commit wins and the ones it superseded
are skipped. main is linear, so the build that wins already contains them. Use a
manual run if a specific commit has to reach testers.

The signed APK is attached to the run as an artifact for 14 days.

The app does not contain the Firebase SDK and needs no `google-services.json`.
App Distribution is a delivery channel around the binary, not a library inside
it; nothing in `src-tauri` knows Firebase exists.

## Package name

The Android application ID is:

```text
com.quantumbox.library
```

That is not the identifier in `tauri.conf.json`. Desktop is frozen on
`com.quantumbox.library.client` — changing it would orphan every installed
copy's macOS settings, Keychain items, and Windows uninstall entry — so Android
overrides it in `src-tauri/tauri.android.conf.json`, which Tauri merges over the
base config for Android builds only, the same way iOS does. Nothing else in the
file needs to be repeated; the merge is per key.

Firebase binds the package name when the Android app is registered, and Play
would bind it permanently later, so it is worth getting right now rather than
after testers have it installed.

## Signing

Tauri's Gradle template declares no release `signingConfig`, so
`tauri android build` produces `app-universal-release-unsigned.apk` and the
workflow finishes it afterwards. `apksigner`, not `jarsigner`: an APK carrying
only a v1 JAR signature will not install on Android 11 and later. `zipalign`
runs first, because `apksigner` preserves whatever alignment it is handed.

**There is no Play App Signing in this path, so this key is the app signing
key.** Replacing it makes every installed copy refuse the update with
`INSTALL_FAILED_UPDATE_INCOMPATIBLE`, and every tester has to uninstall and
reinstall. It is as unrecoverable as the desktop updater key in
[`desktop-release.md`](./desktop-release.md), and unlike a Play upload key
nobody can reset it. Back it up outside CI.

Signing after the build rather than through Gradle also keeps the generated
project untouched. Patching `gen/android/app/build.gradle.kts` in CI to add a
`signingConfig` would have to be redone against every Tauri CLI template change;
the signing step instead fails loudly if the APK ever arrives already signed.

## Current state

Set up on 2026-09-09 and wired end to end:

| Thing | Value |
| --- | --- |
| Firebase project | `planet-library` (`planet-library-1ca62`), under the `quantum-box.com` GCP organization |
| Android app | `com.quantumbox.library`, nickname *Library Android* |
| App ID | `1:845752196507:android:584dd46517206a5d67553e` |
| Tester group | alias `testers` |
| Service account | `firebase-adminsdk-fbsvc@planet-library-1ca62.iam.gserviceaccount.com` |
| Signing key | PKCS12, alias `library-android`, RSA 4096, SHA-256 fingerprint `9F:85:EF:...:23:36` |

All five secrets and the App ID variable are configured on this repository, and
the whole pipeline — build, `zipalign`, `apksigner`, upload — was run once from
a laptop against these credentials before the workflow was merged. That first
release (version code 1, 33.5 MB, `com.quantumbox.library`) sits in App
Distribution with no group attached: it was a pipeline check, not something
testers were notified about.

The service account is the one Firebase creates for the Admin SDK rather than a
purpose-made one, because granting a fresh service account an IAM role needs the
Google Cloud console, which asks for a passkey this project's CI setup cannot
supply. Its default role turned out to cover App Distribution reads and writes,
verified by listing and adding a tester before anything depended on it. If it is
ever narrowed, the fix is a dedicated service account with **Firebase App
Distribution Admin**.

Google Analytics is deliberately off: App Distribution does not use it, and
turning it on would add data collection the app does not otherwise do. The app
carries no Firebase SDK, so no `google-services.json` is checked in.

Google Play is not in the picture. The `Quantum Box, Inc.` developer account was
closed in November 2024 under Google's unused-account policy and cannot be
revived, and the `FANG Inc.` account does not grant this user permission to
create an app. Publishing there means a new developer registration, which is a
separate decision.

The sections below are what to redo if any of this has to be rebuilt.

## Required setup

### 1. Create the signing keystore

```bash
keytool -genkeypair -v \
  -keystore library-android-key.p12 \
  -alias library-android \
  -keyalg RSA -keysize 4096 -validity 10000 \
  -dname 'CN=Quantum Box, O=Quantum Box, C=JP'
```

`keytool` writes PKCS12 by default, which is the format to want; a PKCS12
keystore has no separate key password, so `ANDROID_KEY_PASSWORD` below holds the
same value as `ANDROID_KEYSTORE_PASSWORD`. The two secrets stay separate so a
legacy JKS keystore with a distinct key password also works.

If there is no JDK on `PATH`, Android Studio's bundled one has `keytool`:
`/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin`.

Put the keystore and its password somewhere durable before going further. This
one cannot be reissued.

### 2. Register the Android app in Firebase

1. Firebase console → the project for this app (create one if there is none).
2. **Project settings → Your apps → Add app → Android**, package name
   `com.quantumbox.library`. A nickname is optional.
3. Skip the `google-services.json` download and the SDK steps; App Distribution
   does not need them.
4. Copy the **App ID** — it looks like `1:123456789012:android:abcdef0123456789`.
5. **Release & Monitor → App Distribution**, accept the terms, then create a
   tester group. Its *alias* (not its display name) is what the workflow passes
   to `--groups`; `testers` is the default the workflow assumes.

### 3. Create the service account

1. Google Cloud console for the same project → **IAM & Admin → Service
   Accounts**, create one and give it a JSON key.
2. Grant it the **Firebase App Distribution Admin** role on the project.

### 4. Set the secrets and variables

| Name | Kind | Value |
| --- | --- | --- |
| `ANDROID_KEYSTORE_BASE64` | secret | The keystore file, base64 encoded |
| `ANDROID_KEYSTORE_PASSWORD` | secret | The keystore password |
| `ANDROID_KEY_ALIAS` | secret | The key alias, e.g. `library-android` |
| `ANDROID_KEY_PASSWORD` | secret | The key password (same as the keystore password for PKCS12) |
| `FIREBASE_SERVICE_ACCOUNT_JSON` | secret | The whole service account JSON key, verbatim |
| `FIREBASE_ANDROID_APP_ID` | variable | The Firebase App ID from step 2 |
| `FIREBASE_TESTER_GROUPS` | variable | Optional; comma-separated group aliases, defaults to `testers` |

The App ID is an identifier rather than a credential, so it is a repository
variable — visible in logs, which is what you want when a distribution fails.

```bash
base64 -i library-android-key.p12 | pbcopy
```

```bash
gh secret set ANDROID_KEYSTORE_BASE64
gh secret set ANDROID_KEYSTORE_PASSWORD
gh secret set ANDROID_KEY_ALIAS
gh secret set ANDROID_KEY_PASSWORD
gh secret set FIREBASE_SERVICE_ACCOUNT_JSON < service-account.json
gh variable set FIREBASE_ANDROID_APP_ID
```

Until the four signing secrets exist the workflow skips on `main` with a warning
rather than failing the branch; a manual run fails, because someone asked for a
distribution and did not get one. The Firebase credentials are checked the same
way, except on a `build_only` run, which does not need them — useful for
producing an APK to sideload before the Firebase side exists.

### 5. Invite testers

App Distribution mails each tester an invitation they have to accept once. From
then on a build reaches them as a notification, and they install it through the
Firebase App Tester app or a direct download link. Android will ask them to
allow installs from that source the first time.

## Launcher icon

`tauri android init` renders the Gradle project from the CLI's own template,
which ships Tauri's placeholder launcher icons, and nothing in the CLI copies
`src-tauri/icons/android` into it — `tauri icon` writes into the generated
`res` directory directly, and only when the project already exists. From a clean
checkout, which is every CI run, that means the placeholder unless something
puts the real icons there. `scripts/sync-android-app-icon.mjs`
(`npm run tauri:android:icon`) does, copying every `mipmap-*` folder over the
generated `src-tauri/gen/android/app/src/main/res`. It runs after every
`tauri android init`, including inside `npm run tauri:android:init`, and fails if
the generated project has a resource the icons directory cannot replace rather
than leaving that one as Tauri's.

`mipmap-anydpi-v26/ic_launcher.xml` is the adaptive icon and has no counterpart
in the template, so it is added rather than overwritten. It is what Android 8
and later actually draw; the density PNGs are the fallback for older releases.

## Failure modes worth recognizing

- **`No APK was produced by the Android build`** — Gradle failed earlier; read
  its output above this step rather than this message.
- **`The runner image did not export ANDROID_NDK_LATEST_HOME`** — the runner
  image dropped its bundled NDK. Pin an explicit `$ANDROID_HOME/ndk/<version>`
  in that step and install it with `sdkmanager`.
- **Linker errors naming `aarch64-linux-android*-clang`** — `NDK_HOME` did not
  reach Gradle. Tauri reads that variable and no other.
- **`zipalign: command not found`** — the build-tools directory did not make it
  onto `PATH`; neither tool is there by default on the runner.
- **`Keystore file ... not found` or a wrong-alias error from apksigner** —
  `ANDROID_KEY_ALIAS` does not match the keystore. `keytool -list -keystore
  <file>` shows the aliases it actually has.
- **`Expected an unsigned APK`** — a Tauri CLI upgrade started generating a
  release `signingConfig`. Move the signing into Gradle rather than signing what
  Gradle already signed.
- **`Failed to fetch app information` / `Request to ... failed`** — the App ID in
  `FIREBASE_ANDROID_APP_ID` is wrong, or the service account is on a different
  project.
- **`Missing permissions` / `403` from the CLI** — the service account does not
  have Firebase App Distribution Admin on that project.
- **`Group with alias ... not found`** — `--groups` takes the group *alias*, not
  its display name.
- **`INSTALL_FAILED_UPDATE_INCOMPATIBLE` on a tester's phone** — the APK was
  signed with a different key than the copy they have installed. There is no fix
  from this side; they uninstall first, and the key has to go back to the
  original.
- **`INSTALL_FAILED_VERSION_DOWNGRADE`** — the version code went backwards, e.g.
  a manual run with a smaller `version_code`. Re-run with a larger one.
- **The launcher shows the Tauri logo** — the icon copy did not run. `tauri
  android build` on a project generated without it keeps the placeholder; re-run
  `npm run tauri:android:icon` and rebuild.

## Local reproduction

```bash
cd apps/client
npm ci
npm run tauri:android:init
npm run tauri -- android build --apk --target aarch64 \
  -c '{"bundle":{"android":{"versionCode":1}}}'
```

If `JAVA_HOME` points at an old or missing JDK, use Android Studio's bundled JBR
as [`app-platforms.md`](./app-platforms.md#android) describes.

Signing by hand, with `zipalign` and `apksigner` from
`$ANDROID_HOME/build-tools/<version>/`:

```bash
zipalign -p -f 4 \
  src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk \
  aligned.apk
apksigner sign --ks library-android-key.p12 --ks-key-alias library-android \
  --out app-universal-release.apk aligned.apk
```

Distributing by hand needs the same service account:

```bash
GOOGLE_APPLICATION_CREDENTIALS=service-account.json \
npx firebase-tools@15 appdistribution:distribute app-universal-release.apk \
  --app "$FIREBASE_ANDROID_APP_ID" --groups testers
```
