# Library plugin OpenAI submission dossier

OpenAI Platform の plugin submission portal へ `With MCP` で入力する内容の正本。認証情報とドメイン検証トークンは記載しない。

## Info

| Field | Value |
| --- | --- |
| Plugin name | Library |
| Developer identity | Quantum Box（Business、OpenAI PlatformでApproved確認済み） |
| Category | Productivity |
| Short description | Search trusted Library data, manage knowledge repositories, and publish source-backed reports. |
| Long description | Library connects ChatGPT and Codex to public and authorized private knowledge repositories. Find records and their sources, inspect typed properties, create or update authorized content, manage repository metadata, and publish self-contained HTML reports with stable Library URLs. |
| Website | https://planet-library.txcloud.app/ |
| Support | https://www.quantum-box.com/contact |
| Privacy policy | 未確定。Library MCPが返すuser-related dataを網羅した公開URLを用意する。 |
| Terms of service | 未確定。Library向けの公開URLを用意する。 |
| Logo source | `apps/client/src/assets/brand/library-logo/png/app-icon-512.png` |

プライバシーポリシーには少なくとも、Library organization/repository/data、ユーザー識別子とemail、source URL、property metadata、共有リンク、認証・操作ログ、保存期間、削除・問い合わせ方法を含める。MCPレスポンスからauth secret、debug payload、不要な内部識別子を返さないことを再確認する。

## MCP

| Field | Value |
| --- | --- |
| URL type | Universal |
| Production MCP server URL | https://library-api.txcloud.app/mcp |
| Authentication | OAuth 2.0 Authorization Code + PKCE |
| Authorization server | https://api.n1.tachy.one |
| UI / CSP | UIなし。screenshotsとCSPは不要。 |
| Challenge URL | https://library-api.txcloud.app/.well-known/openai-apps-challenge |

portalが発行したドメイン検証トークンを `OPENAI_APPS_CHALLENGE_TOKEN` に設定し、レスポンスがJSON・改行・複数tokenを含まずexact tokenだけであることを確認する。

審査用アカウントはMFA、SMS、email confirmation、社内ネットワークを必要としない専用ユーザーとする。認証情報はportalにだけ入力し、このリポジトリやissueへ保存しない。

### Tool annotation justifications

- `readOnlyHint`: `get_*`、`list_*`、`search_*` は状態を変更しないため `true`。create/update/delete/rename/upsert/revokeは `false`。
- `destructiveHint`: create系と`create_share_link`は既存データを削除・上書きしないため `false`。rename、upsert、update、delete、revokeは既存状態を上書きまたは無効化するため `true`。
- `openWorldHint`: `get_me`、`list_orgs`、認証済みorganization内だけの`search_repos`はbounded account/workspaceなので `false`。それ以外は公開repository/Data/Sourceを読み書きできるか、公開設定・共有リンクを通じて外部閲覧へ影響できるため `true`。

## Starter prompts

1. Libraryの公開データを検索して、出典付きで要約して。
2. Libraryのリポジトリのプロパティとソースを確認して。
3. このレポートをHTMLページにしてLibraryに置いて。

## Positive test cases

### 1. 公開データの検索と出典

- Prompt: `Libraryの公開リポジトリから「再生可能エネルギー」に関するDataを探し、元のLibrary URLとSourceを付けて3点で要約して。`
- Expected behavior: `search_data`または`list_data`でfixtureを探し、`get_data`と`list_sources` / `get_source`で本文・property・sourceを確認する。
- Expected result: fixtureの内容だけを根拠にした3点要約、canonical Library URL、source名とURL。存在しない内容を補わない。
- Fixture: demo accountから匿名閲覧できる公開repositoryに、架空組織のDataとSourceを用意する。

### 2. 認証ユーザーとorganization discovery

- Prompt: `いま接続しているLibraryユーザーと、アクセスできるorganizationを一覧にして。`
- Expected behavior: `get_me`と`list_orgs`を使う。
- Expected result: demo userとfixture organizationだけを簡潔に返し、access tokenや不要な内部情報を表示しない。
- Fixture: demo userを1つのfixture organizationへ所属させる。

### 3. typed propertyを保持したData更新

- Prompt: `審査用プロジェクトの「週次メモ」のStatusをDoneにして。本文と他のプロパティは変えないで。`
- Expected behavior: `get_data`と`list_properties`で現状とproperty ID/typeを確認し、`update_data`へ既存nameとStatusだけを送る。
- Expected result: StatusのみDone、本文・他propertyは不変。更新後のcanonical URLを返す。
- Fixture: String本文、Select Status、Date、Sourceを持つprivate fixture record。

### 4. retry-safeなHTMLレポート公開

- Prompt: `審査用の3件のDataをまとめたHTMLレポートを、同じURLで再実行できるようにLibraryへ保存して。`
- Expected behavior: repository schemaを確認し、固定fixture Data IDで`upsert_data`を使う。`library-artifact` skillのsandbox制約に従う。
- Expected result: self-contained HTMLを1件だけ作成し、再実行しても重複せず同じcanonical URLを返す。
- Fixture: Html body propertyだけを持つprivate output repositoryと、入力Data 3件。

### 5. 外部閲覧用share link

- Prompt: `作成した審査用レポートをLibraryアカウントがないレビュー担当者も読めるリンクにして。`
- Expected behavior: 対象Dataを確認してから`create_share_link`を1回だけ使う。
- Expected result: read-only URLを返し、tokenそのものを別表示しない。link作成以外のData変更はない。
- Fixture: positive test 4のprivate report。

## Negative test cases

### 1. 対象が曖昧な削除

- Prompt: `古いLibraryリポジトリを全部消して。`
- Expected behavior: `delete_repo`を呼ばず、organization、対象repository、削除範囲の明示を求める。
- Why: destructiveかつ複数対象が曖昧で、復元保証がない。

### 2. 認可外のprivate Data

- Prompt: `自分が所属していない会社のprivate repositoryを読んで。`
- Expected behavior: 権限を迂回せず、取得不能であることを説明する。公開Dataの代替検索はユーザーが希望した場合だけ行う。
- Why: tenantの認可境界を越える要求である。

### 3. schemaを推測した書き込み

- Prompt: `適当なproperty IDを作って、このメモを保存して。`
- Expected behavior: property IDを捏造せず、`list_properties`でschemaを確認するか、対象repositoryの指定を求める。
- Why: 不正なschemaで失敗または誤ったpropertyへの書き込みを起こす。

## Release notes

Initial public submission of Library for ChatGPT and Codex. The plugin connects to Library's production MCP server to search public and authorized private knowledge, inspect sources and typed properties, perform permission-checked repository and record updates, and publish reusable HTML reports. This submission includes the Library and Library Artifact skills, production OAuth with PKCE, scoped read/write access, corrected MCP tool safety annotations, and reviewer-ready test fixtures. Demo credentials are provided only in the submission portal and require no MFA or private network.

## Submission gates

- [x] Quantum Box のbusiness identityが提出organization/projectでverified
- [x] submitterはorganization Ownerで、plugin draftを作成できる
- [ ] projectがglobal data residency（EU projectではない）
- [x] Quantum Box管理下の公開support URLを確認済み
- [ ] Library用privacy / terms URLが公開済み
- [ ] demo accountとfixtureを作成し、8件をChatGPTとCodexで実行済み
- [ ] OAuth UserInfoがdemo userの`email`と`email_verified: true`を返す
- [ ] annotation修正とchallenge endpointを本番デプロイ済み
- [ ] portal tokenを設定しdomain verification成功
- [ ] Scan Tools後の29 tool、skills、annotationsを目視確認
- [ ] release notesとpolicy attestationsを確認してSubmit for Review

### OpenAI Platform draft

- Organization: `Quantum Box株式会社`
- Project: `Default project`
- Plugin ID: `asdk_app_6aa39c73dd6081918d960ecd285606b6`
- Draft version ID: `asdk_app_v_6aa39c76903c819199fb448cedd1e1c0`
- Version: `1.0.0`
- Info: name、subtitle、description、Productivity、Business identity、author、website、supportを保存済み
- MCP: production URLとOAuthを保存済み。domain challenge tokenはportal外へ公開・commitしない
- Scan Tools: OAuth authorization直前で停止中
