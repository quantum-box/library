# App Platform Builds

Photon ships from the same React/Tauri codebase to web, desktop, Android, and
iOS. The repo treats each branded product as an app profile plus a deployment
topology:

- App profile: labels, navigation, record defaults, storage keys, and sync room
  names in `src/app/kitConfig.ts`.
- Frontend Worker: a required frontend-side edge companion for health checks,
  `/ws`, and future frontend-owned API glue.
- Application server: the canonical Rust API server, or an external API that
  implements the same record contract.
- Sync backend: either the Rust server WebSocket endpoint or Cloudflare Durable
  Objects behind the frontend Worker.

Production app builds point at this app's own sync Worker:

```text
wss://library-client-sync.quantum-box.workers.dev/ws
```

It is defined by `apps/client/wrangler.jsonc` and shipped by the
`library-client-sync` Cloud App in the repo-root `tachyon.yaml`, which runs
`wrangler deploy` against that same config on every push to `main` that touches
the Worker's bundle. `npm run worker:deploy` stays available for a manual
deploy, but is no longer how production is kept current: while the Worker was
hand-deployed the screen followed `main` through its own Cloud App and the
Worker did not, and on 2026-09-08 production still ran the 09-05 build. Every
Live room answered the WebSocket upgrade with 409, and because a browser cannot
read a failed handshake's status the data editor simply said it was offline and
retried forever.

Production used to point at `photon-sync`, which is Photon's own deployment
rather than this app's: Library did not control when it was redeployed, and the
build serving it had been frozen since 2026-05-05.

The endpoint is named in three places, and all three have to agree or one shell
loses realtime sync: `.env.production` (local and desktop builds), the
`library-client` app in the repo-root `tachyon.yaml` (hosted web builds), and
the `connect-src` allowlist in `src-tauri/tauri.conf.json` -- the desktop shell
refuses a WebSocket to an origin the CSP does not name.

Moving the endpoint moves where Durable Object room state lives, but does not
need the old rooms carried over. Both Yjs surfaces persist locally
(`IndexeddbPersistence` in `src/lib/yjs/yjsProvider.ts` and
`src/lib/docs/docYjs.ts`) and send their full state on connect, so an empty room
rehydrates from the first client to reach it.

Web release candidates must pass `npm run build` and keep API/WebSocket
endpoints explicit through Vite environment variables or `src/app/kitConfig.ts`.
Tauri release candidates must pass the CI Linux smoke build and should use
`tauri-local-file-cache` for desktop attachment byte references. Synced
attachment metadata must not contain absolute local filesystem paths.

The macOS desktop shell runs its tabs as child WebViews of a single window; see
[`macos-window-tabs.md`](./macos-window-tabs.md).

The desktop shell has no address bar, so `⌘L` (`Ctrl+L` on Windows and Linux)
copies the address of the current route instead
(`src/lib/desktop/useCopyPageUrl.ts`). It is bound at the root layout, so the
public reader and the sign-in gate answer it as well as the workspace.

Two things decide whether it binds and what it copies. The shell is Tauri on
iOS and Android too, so the key is gated on the target OS
(`useDesktopShell`), not on the presence of the Tauri internals; web builds
leave it to the browser, which already shows and copies the same URL. And the
shell serves the app from `tauri://localhost`, which no one outside the app can
open, so `src/lib/shareUrl.ts` rewrites that origin onto
`appKitConfig.app.publicOrigin` (`https://planetlibrary.txcloud.app`,
overridable with `VITE_LIBRARY_CLIENT_ORIGIN`). A hosted or dev-server origin is
copied as it stands.

Mobile release candidates must also pass the phone-viewport browser smoke:

```bash
npm run test:e2e:mobile
```

Nothing may pan the app sideways. The page itself never scrolls horizontally,
and anything wider than the screen scrolls inside a pane that owns it — the
repository tab strip, the board, a chat table — each carrying
`overscroll-x-contain` so the drag stops there. Three things break that rule if
left alone, and each has a fix worth knowing about:

- **Long unbroken text.** `body` sets `overflow-wrap: anywhere`. `break-word`
  is not enough: only `anywhere` shrinks an element's min-content width, and it
  is the min-content width that pushes a flex or grid item off the screen. A
  190-character data title used to widen a repository card by 937px. Anything
  that would rather clip opts out with `truncate`.
- **A grid whose columns are only declared at a breakpoint.** The implicit
  column is `auto`, which grows to max-content, and a `truncate` descendant's
  max-content is the whole untruncated string. Home's activity card came out
  1290px wide on a 402px screen. Every such grid names
  `grid-cols-[minmax(0,1fr)]` at the base width. `1fr` on its own is not
  enough either — its minimum is `auto`, so write `minmax(0,1fr)`.
- **HTML artifacts.** See `src/lib/html/artifactDocument.ts`: an artifact is
  someone else's whole document and was not written for a phone, so the frame
  injects a viewport meta and a `max-width` floor ahead of the author's own
  markup. Measured on an iPhone 17 Pro, that takes the embedded document from
  757px to the 386px frame.

`mobile.spec.ts` asserts the rule with a 190-character title, on the list and
on the data page.

The phone shell is an app frame rather than a document, and two rules keep it
that way. `index.html` pins the viewport (`maximum-scale=1, user-scalable=no,
viewport-fit=cover`) so a focused input cannot zoom the layout out from under
the user, and `src/index.css` fixes **`#root`** to the layout viewport with
`overscroll-behavior: none` on `body`. Nothing outside a pane may scroll: a
screen that makes the document itself overflow is a bug, and `mobile.spec.ts`
asserts it.

The frame is `#root` and not `body` on purpose. Radix locks scrolling for every
menu, dialog and popover by injecting
`body[data-scroll-locked] { position: relative !important; padding-top: … }`
through `react-remove-scroll-bar`. A `body` carrying `position: fixed` and the
safe-area insets loses both the moment anything opens: the body grows to the
height of the popover and the app slides up behind the notch. Put nothing the
layout depends on on `body`.

On a phone the workspace navigation lives in a drawer behind the app bar rather
than in the sidebar, and views that cannot reflow — the repository data table
and its public counterpart, above all — swap to a card layout through
`useIsMobileViewport`. Anything a phone can only reach by tapping has to have a
tap target: the command palette is opened from the drawer, not just by `⌘K`.

The phone/desktop line is width **and** height. A phone in landscape is 874x402:
wider than the desktop breakpoint and far too short for a sidebar, a header and
a pane, so `md` is redefined in `src/index.css` as
`(min-width: 768px) and (min-height: 500px)` and every `md:` in the app follows
it. `MOBILE_VIEWPORT_QUERY` and the `.detail-panel` rules state the same line
and have to move with it. 500px clears every phone in landscape (up to ~440)
and sits below every tablet (744+) and the desktop window's own 640px minimum.

Screen chrome is not free on a phone. The shell's app bar already carries the
Library mark, the drawer and the account menu, so a page that also draws a
desktop header spends a tenth of the screen repeating it — `LibraryHome` hides
both its header and its tab strip below `md` for that reason. Popovers need the
same care in the other direction: the account menu is the tallest in the app,
and without a `max-height` and `collisionPadding` it does not fit a phone,
which drags the whole page up behind the notch as the browser tries to reveal
it. `useSafeAreaInsets` exists for that, because Radix positions from
JavaScript and cannot read `env()`.

### iOS shell

The WebView is stretched over the window in `src-tauri/src/ios_webview.rs`.
Tauri sizes a webview to the window's *inner* size, and `tao` reports that as
the safe area on iOS, so the WebView comes out 96pt shorter than the screen
while still sitting at the top of it. `env(safe-area-inset-*)` keeps reporting
the device's real insets, so the stylesheet pads `body` on top of a frame that
already lost the same space and the app stops 130pt short of the bottom of the
screen. The module resets the frame, gives it a flexible autoresizing mask so
rotation carries through, and turns off the scroll view's automatic content
inset so UIKit does not put the margin back as scroll offset.

`bundle.iOS.minimumSystemVersion` is **16.4**, not the Tauri default. Tailwind
v4 emits `@property`, `color-mix()` and `oklch()`, which Safari only understands
from 16.4; `dvh`, `:has()` and `overscroll-behavior` land around the same
releases. A lower floor ships a build that renders wrong rather than one that
refuses to install.

Element fullscreen does not exist on iPhone at all — `requestFullscreen` is
absent rather than failing — so `HtmlArtifactEditor` falls back to a fixed
overlay and its full-screen button means the same thing on every platform.

Every push to `main` builds and ships both mobile shells: the iOS archive goes
to TestFlight and the Android APK to Firebase App Distribution, which also keeps
the Tauri mobile wrapper, Rust command bridge, frontend build, and generated
mobile projects in sync with the shared app shell.

Release following for apps that should avoid the npm registry is documented in
[`release-following.md`](./release-following.md).

## App Profiles

When creating another app from Photon, start by changing only
`src/app/kitConfig.ts`:

- `app.id`, `displayName`, and `storageNamespace`
- `workspace.name`, navigation, projects, and users
- `records.identifierPrefix` and `defaultProject`
- `chat.productName` and disclaimer copy
- explicit storage and sync keys

Do not scatter product names, storage keys, or endpoint paths into components.
The UI should consume `appKitConfig`, and runtime wiring should stay behind the
same config helpers.

## Deployment Topologies

Use `VITE_PHOTON_DEPLOYMENT_MODE` to document the intended topology for a build:

| Mode | Frontend Worker | Sync default | App API default |
| --- | --- | --- | --- |
| `local` | Cloudflare Worker dev server | Rust server `/ws` | Rust server |
| `cloud` | Cloudflare Workers | Durable Object relay | Rust server or external API |
| `onprem` | `workerd` container | Rust server `/ws` | Rust server |

The Frontend Worker is always part of the frontend platform contract. In cloud
deployments it runs on Cloudflare Workers. In on-premise deployments, run the
same Worker boundary through `workerd` (or a compatible Workers runtime) next to
the static frontend assets.

Recommended on-premise shape:

```text
browser / desktop / mobile
  -> TLS ingress (Caddy, Nginx, Traefik, or appliance LB)
  -> frontend bundle + workerd Frontend Worker
       /api/health
       /ws
       static assets
  -> Rust app server
       /api/records
       optional /ws sync
  -> durable database volume or managed on-prem database
```

For a strict offline/on-prem install, keep sync on the Rust server and persist
the server database outside the container. For a connected private-cloud install
that is allowed to reach Cloudflare, the sync backend can still be overridden to
`cloudflare-durable-object`.

Useful environment variables:

```bash
VITE_PHOTON_DEPLOYMENT_MODE=onprem
VITE_PHOTON_FRONTEND_WORKER_RUNTIME=workerd
VITE_PHOTON_SYNC_BACKEND=rust-server
VITE_PHOTON_API_BASE_URL=https://photon.example.internal
VITE_PHOTON_SYNC_WS_URL=wss://photon.example.internal/ws
```

## Desktop

Build a local desktop bundle:

```bash
npm run tauri:build
```

Create a GitHub desktop release by pushing a semver tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The `Release` workflow builds desktop bundles for macOS, Linux, and Windows and
publishes the GitHub Release when all platform builds finish. It also runs
automatically after `CI` passes on `main`: if `package.json` has a version that
does not already have a matching `vX.Y.Z` tag, the workflow creates that tag and
publishes the desktop release.

## Android

The Android project lives under `src-tauri/gen/android`.

```bash
npm run tauri:android:dev
npm run tauri:android:build
npm run tauri:android:build -- --target aarch64 --apk
```

The generated Android manifest includes Internet permission so the bundled app
can connect to the Cloudflare sync Worker. The application ID is
`com.quantumbox.library`, set in `src-tauri/tauri.android.conf.json` so desktop
can stay on its frozen `com.quantumbox.library.client`.

Release builds are distributed to testers by
[`android-firebase-distribution.md`](./android-firebase-distribution.md).

If `JAVA_HOME` points at an old or missing JDK, use Android Studio's bundled
JBR for the build:

```bash
JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" \
PATH="/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin:$PATH" \
npm run tauri:android:build -- --target aarch64 --apk
```

## iOS

The iOS project lives under `src-tauri/gen/apple`.

```bash
npm run tauri:ios:dev
npm run tauri:ios:build -- --target aarch64-sim
npm run tauri:ios:build:sim
```

CI uses `npm run tauri:ios:build:sim` as the unsigned simulator smoke build.
This catches regressions in the shared React bundle, Tauri mobile wrapper, and
Rust command bridge without requiring App Store signing.

Code signing uses the Quantum Box Apple development team configured in
`src-tauri/tauri.conf.json`. Set `APPLE_DEVELOPMENT_TEAM` in CI or the shell to
override it for a different team.

Every push to `main` also builds a signed archive and uploads it to TestFlight;
see [`ios-testflight.md`](./ios-testflight.md) for the credentials it needs and
how build numbers are assigned. The generated Xcode project
(`src-tauri/gen/apple/library-client.xcodeproj`) is there for device archives
and signing done by hand.
