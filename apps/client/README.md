# Library Client

Library の React クライアントです。データの境界は GitHub に寄せ、日常の編集体験は Notion のようにページ中心で扱います。

```text
Organization
└── Repository
    ├── Data (page / structured row)
    └── Property (schema)
```

ホームでは最近編集したページから作業を再開し、サイドバーでは `organization/repository` を選びます。Repository 配下では同じ Data を table / board / workflow / document の各ビューで扱います。

## UI とランタイム

| 対象 | 実装 |
|---|---|
| Web | React 19 + Vite 8 |
| Desktop | Tauri v2 (macOS / Windows / Linux) |
| Mobile | Tauri v2 (iOS / Android) |
| UI system | `@tachyon-sdk/native-ui` + Tailwind CSS v4 |
| Routing / data grid | TanStack Router / TanStack Table |
| Rich text | BlockNote |
| Local-first | PGlite + Yjs + IndexedDB |
| Durable / realtime | Photon Engine + Photon Live |

Native UI は固定 commit の codeload tarball を利用します。通常の npm install に強い GitHub 認証は不要です。

## 認証

通常のサインインは次の順で行います。

1. Cognito `USER_PASSWORD_AUTH`
2. Cognito `GetUser`
3. Library GraphQL `signInWithPlatform(allowSignUp: true)`
4. Library user id を含むセッションを保存

期限の近いアクセストークンは `REFRESH_TOKEN_AUTH` で更新します。認証後はユーザー単位の PGlite / IndexedDB 名前空間へ切り替わるため、一度画面を再初期化します。手入力トークンは `VITE_ENABLE_DEV_TOKEN_AUTH=true` のローカル開発時だけ表示されます。

## 公開ルート (read-only)

public repository はサインインなしで閲覧できます。

| ルート | 内容 |
|---|---|
| `/public/{org}/{repo}` | repository の Data 一覧（読み取り専用） |
| `/public/{org}/{repo}/{dataId}` | Data 1件のページ表示（本文は編集不可） |

- `/public/...` だけが `AuthGate` の外側で描画されます。ワークスペース側の provider（records / databases / views / attachments）は読み込まれません。
- この2つのルートからの API 読み取りは常に匿名で行われ、セッションがあっても `Authorization` ヘッダーを付けません。サインイン中のオーナーが自分の repository を開いても、匿名の訪問者と同じ内容が表示されます。
- repository が private の場合、`is_public` と API の 403 の両方で弾き、サインイン導線を表示します。存在しない場合は 404 として区別します。
- org username が `public` の organization はこのルートに隠されます（静的セグメントが `$organization` より優先されるため）。アプリ内の他のルートからは通常どおり開けます。

## ローカル開発

```bash
cd apps/client
cp .env.example .env.local
npm ci

# Web
npm run dev

# Desktop
npm run tauri:dev

# Checks
npm run type-check
npm run test
npm run test:e2e
```

`.env.local` には最低限 `VITE_COGNITO_CLIENT_ID` を設定してください。ローカル Library API の既定値は `http://127.0.0.1:50053` です。

## ビルド

```bash
# Web
npm run build

# Desktop
npm run tauri:build

# Native project generation (初回のみ)
npm run tauri:android:init
npm run tauri:ios:init

# Mobile
npm run tauri:android:build
npm run tauri:ios:build
npm run tauri:ios:build:sim
```

Tauri の生成物は `src-tauri/gen/android` と `src-tauri/gen/apple` に作られます。Android Studio / Xcode と各 SDK のセットアップは Tauri のプラットフォーム要件に従います。

## アプリアイコン

全プラットフォームのアイコンは `src-tauri/icons/icon-manifest.json` から生成します。元は `src/assets/brand/library-logo/badge-dark.svg` で、プラットフォームごとの要件に合わせた派生を同じディレクトリに置いています。

| ソース | 用途 |
|---|---|
| `icon-source.svg` | macOS / Windows / Linux。バッジを macOS のアイコングリッド（1024 中 824）に収め、周囲は透明 |
| `bg_color` (`#07172f`) | iOS。透明部分をブランドネイビーで埋めて全面塗りにする |
| `icon-source-android-bg.svg` | Android adaptive icon の背景レイヤー |
| `icon-source-android-fg.svg` | 同・前景レイヤー。safe zone (61%) に収まるよう縮小済み |
| `icon-source-android-monochrome.svg` | Android 13 のテーマアイコン。単色＋実際の透明な隙間 |

```bash
npx tauri icon src-tauri/icons/icon-manifest.json
```

生成後、iOS のアイコンからアルファチャンネルを落とします（App Store はアルファ付きアイコンを弾きます）。

```bash
python3 -c "from PIL import Image; import glob; [ (lambda im: (lambda f: (f.paste(im, mask=im.split()[3]), f.save(p)))(Image.new('RGB', im.size, (7,23,47))))(Image.open(p).convert('RGBA')) for p in glob.glob('src-tauri/icons/ios/*.png') ]"
```

マニフェストの `android_fg_scale` は現行 CLI では反映されないため、前景の縮小はソース SVG 側で行っています。

## 設定上の注意

- Production CSP は現在の Cognito、Tachyon API、Library API、Photon Live の origin に限定しています。接続先を変えた場合は `src-tauri/tauri.conf.json` も更新してください。
- WebView 内の認証保存は差し替え可能な `AuthTokenStorage` を通します。現時点の既定実装は `localStorage` です。
- Chat と Engine の production endpoint は環境変数で明示してください。Web 開発時の相対 `/api/*` は Vite proxy 経由です。
