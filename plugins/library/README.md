# Library plugin

Claude Code と Codex から Library の所属orgの一覧取得、公開・許可された非公開データの検索・要約、認証済みユーザーに許可された更新操作、HTML アーティファクトの Library への保存を行うプラグインです。HTTP MCP の接続設定と、両クライアントで共有する `library` スキルを含みます。ローカルの MCP サーバーや Library CLI のインストールは不要です。

## インストール

以下の GitHub 経由の手順は、このディレクトリと marketplace 定義がデフォルトブランチに公開された後に使用できます。開発中は後述のローカル手順を使ってください。

### Claude Code

Claude Code 内で実行します。

```text
/plugin marketplace add quantum-box/library
/plugin install library@library
```

インストール後は新しいセッションを開き、`/mcp` で Library の接続を確認します。`/library:library` でデータ操作のスキルを、`/library:library-artifact` で HTML アーティファクトのスキルを明示的に呼び出せます。

### Codex

プラグイン機能に対応した Codex CLI で実行します。

```bash
codex plugin marketplace add quantum-box/library
codex plugin add library@library
```

インストール後は新しいタスクを開いてください。Codex アプリのプラグイン画面からも、追加した Library marketplace のプラグインを選択できます。

## 使い方

まず「Libraryのorg一覧を取得して」と依頼できます。ログイン後、`get_me` → `list_orgs` → `list_repos` で対象を選べます。organization / repository の slug が分かる場合は添えて依頼します。以下の `example-org` / `handbook` は架空の値なので、自分の対象に置き換えてください。

- 「Libraryの `example-org/handbook` からオンボーディングに関するデータを探し、Data IDを添えて要約して」
- 「Libraryの `example-org/handbook` のプロパティとソースを確認して」
- 「このメモをLibraryの `example-org/handbook` に新しいデータとして保存して」
- 「今週のレポートをHTMLページにしてLibraryの `example-org/artifacts` に置いて」

### HTML アーティファクト

Library が接続されていれば、レポート・ページ・ダッシュボード・プロトタイプなど、通常は Claude の artifact になる成果物を Library の Data として保存し、その URL を返します。手順は `library-artifact` スキルにまとまっています。Data は `property_type: html` の Property を 1 つ持つ repository に置きます。指定がなければ所属 org の `artifacts` という slug の repository を提案し、公開範囲（`is_public`）を確認してから作成します。作成時に付く RichText の `content` property は削除し、Html property だけを持たせます（client は RichText を優先して本文に選ぶため）。同じページの更新では同じ Data ID を再利用するため URL は変わりません。

Library v2 は Html Property を `allow-scripts` だけのサンドボックス iframe で描画します。`allow-same-origin` が無いので localStorage・cookie は例外になり、`allow-forms` / `allow-popups` / `allow-downloads` / `allow-modals` が無いのでフォーム送信・`target="_blank"`・ダウンロード・`alert` は動きません。CSS と JS はインラインにし、画像は data URI か inline SVG にした自己完結の HTML にしてください。`window.claude.*` のような artifact 固有のランタイムは使えません。`srcdoc` は埋め込み元の CSP を継承するため、desktop シェルではインライン script が動かない前提で、JS 無しでも読める内容にします。`POST /mcp` は 1 リクエスト約 2 MiB、Html 値は 3 MiB が上限なので、大きな画像の data URI 埋め込みは失敗します。版履歴は未対応で上書き保存になります。

private repo のページをテナント外に渡すときは `create_share_link` を使います。Data 1 件だけを開く読み取り専用の URL で、token は発行時の 1 回しか表示されません。`list_share_links` で既存のリンクを確認でき、`revoke_share_link` で失効させます。public repo は `/public/<org>/<repo>/<data_id>` が既に匿名で読めるため発行は拒否されます。

`search_data` はリポジトリ内のデータ名の完全一致検索です。本文の全文検索ではありません。名前が分からない場合は `list_data` のページをたどって候補を確認します。スキルは選択したデータを取得してから要約し、根拠のIDや取得できたURLを残します。

## 接続と認証

接続先は `https://library-api.txcloud.app/mcp`（HTTP）です。公開データは匿名で読めます。更新操作には Library アカウントの認証と対象への権限が必要です。Claude Code では `/mcp` から Library を選んで認証し、Codex ではプラグインの認証導線に従ってください。認証後に接続・ツール一覧を再読み込みします。パスワードやトークンをチャットやこのリポジトリに貼り付ける必要はありません。

`0.2.0` 以降は `get_me` / `list_orgs` / `list_repos` / `rename_repo` / `upsert_data` に対応したサーバーで使用します。`0.3.0` は HTML アーティファクトの保存手順を追加したスキルの更新で、サーバー側の要件は `0.2.0` と同じです。`0.4.0` は共有リンク（`create_share_link` / `list_share_links` / `revoke_share_link`）に対応したサーバーを要求します。`0.5.0` はアーティファクト作成手順を `library-artifact` スキルに切り出したスキルの更新で、サーバー側の要件は `0.4.0` と同じです。認証済みのData読み取りには実際のorg IDと利用者の認証情報を渡し、既存のread権限を評価します。`get_data` はMarkdownに加えて、編集に使える型付き `property_data`、正規URL、`record_version` を返します。`upsert_data` は指定したData IDを再利用するため、再試行で別のレコードを作りません。ただし再書き込みや同時更新の競合は防ぎません。現行MCP CRUDは`record_version`を増加させないため、この番号を更新検知・競合確認に使わず、変更内容を再取得して確認します。

ツールが見つからない場合は、接続先APIのデプロイとツール一覧の再読み込みを確認してください。プラグインの更新だけではAPI側のツールは増えません。詳しい対応範囲は [MCP機能監査](https://github.com/quantum-box/library/blob/main/docs/specs/integrations/mcp-coverage.md) にあります。

API key を使う場合は、利用者側のMCPクライアント設定で `Authorization: Bearer <API key>` を設定します。`pk_` キーでは引数にorgがない初期接続・ツール一覧取得にも認証を適用するため、`x-operator-id` にそのキーの **Library organization ID** が必要です。slugや別サービスのtenant IDを使わないでください。秘密値を `.mcp.json` にコミットせず、クライアントの秘密情報管理・環境変数参照を使ってください。CLIでの認証手順は [Library CLI](https://github.com/quantum-box/library/tree/main/apps/cli) にあります。

セルフホストでは、ローカルのプラグインコピーの `.mcp.json` の `url` を自環境の `https://…/mcp` に変更してインストールします。そのサーバーの認証設定も必要です。SSEはNon-GA・既定offのため、このプラグインでは使用しません。

## ローカル開発

Library リポジトリのルートで実行します。

```bash
# Claude Code: ローカルのプラグインを直接読み込む
claude --plugin-dir ./plugins/library

# Codex: ローカルmarketplaceを登録してからインストールする
codex plugin marketplace add .
codex plugin add library@library
```

同名のGit marketplaceを既に登録している場合は、クライアントのmarketplace管理でその登録先を確認し、ローカル版と重複させないでください。

```text
.agents/plugins/marketplace.json       Codex marketplace
.claude-plugin/marketplace.json        Claude Code marketplace
plugins/library/
  .codex-plugin/plugin.json            Codex manifest
  .claude-plugin/plugin.json           Claude Code manifest
  .mcp.json                           共通のHTTP接続設定
  skills/library/SKILL.md              共通スキル（データ操作）
  skills/library-artifact/SKILL.md     共通スキル（HTMLアーティファクト）
  README.md
  LICENSE
```

配布物は `plugins/library` 内で完結します。変更時は両manifestのバージョンを揃えて更新してください。認証済みの内容変更を伴う検証には専用のテストデータを使います。

構造の検証:

```bash
claude plugin validate --strict ./plugins/library
claude plugin validate --strict ./.claude-plugin/marketplace.json
```

公開MCPの接続確認（書き込みなし）:

```bash
curl --fail-with-body https://library-api.txcloud.app/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"library-plugin-check","version":"0.3.0"}}}'

curl --fail-with-body https://library-api.txcloud.app/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
```

manifestの検証と匿名接続の成功は、各クライアントでのOAuthログインや認証済み操作の成功とは別です。

## 参照

- [Library MCP仕様](https://github.com/quantum-box/library/blob/main/docs/specs/integrations/mcp.md)
- [Claude Code plugin仕様](https://code.claude.com/docs/en/plugins-reference)
- [Claude Code marketplace仕様](https://code.claude.com/docs/en/plugin-marketplaces)

MIT License。詳細は同梱の [LICENSE](LICENSE) を参照してください。
