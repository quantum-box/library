# Library plugin

Claude Code と Codex から Library の公開データの検索・要約と、認証済みユーザーに許可された更新操作を行うプラグインです。HTTP MCP の接続設定と、両クライアントで共有する `library` スキルを含みます。ローカルの MCP サーバーや Library CLI のインストールは不要です。

## インストール

以下の GitHub 経由の手順は、このディレクトリと marketplace 定義がデフォルトブランチに公開された後に使用できます。開発中は後述のローカル手順を使ってください。

### Claude Code

Claude Code 内で実行します。

```text
/plugin marketplace add quantum-box/library
/plugin install library@library
```

インストール後は新しいセッションを開き、`/mcp` で Library の接続を確認します。`/library:library` でスキルを明示的に呼び出せます。

### Codex

プラグイン機能に対応した Codex CLI で実行します。

```bash
codex plugin marketplace add quantum-box/library
codex plugin add library@library
```

インストール後は新しいタスクを開いてください。Codex アプリのプラグイン画面からも、追加した Library marketplace のプラグインを選択できます。

## 使い方

organization / repository の slug を添えて依頼します。以下の `example-org` / `handbook` は架空の値なので、自分の対象に置き換えてください。

- 「Libraryの `example-org/handbook` からオンボーディングに関するデータを探し、Data IDを添えて要約して」
- 「Libraryの `example-org/handbook` のプロパティとソースを確認して」
- 「このメモをLibraryの `example-org/handbook` に新しいデータとして保存して」

検索はリポジトリ内のデータ名に対する検索です。本文の全文検索ではありません。スキルは選択したデータを取得してから要約し、根拠のIDや取得できたURLを残します。

## 接続と認証

接続先は `https://library-api.txcloud.app/mcp`（HTTP）です。公開データは匿名で読めます。更新操作には Library アカウントの認証と対象への権限が必要です。Claude Code では `/mcp` から Library を選んで認証し、Codex ではプラグインの認証導線に従ってください。認証後に接続・ツール一覧を再読み込みします。パスワードやトークンをチャットやこのリポジトリに貼り付ける必要はありません。

**現行APIの制限:** `list_data` / `search_data` / `get_data` は認証後も匿名の実行者を使うため、非公開リポジトリのData本文は読めません。認証により利用できるのは許可されたwrite toolsや、認証を参照するメタデータ取得です。非公開Dataの取得には認証済みのLibrary CLI / APIを使用してください。プラグインはAPIの権限やこの制限を変更しません。

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
  skills/library/SKILL.md              共通スキル
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
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"library-plugin-check","version":"0.1.0"}}}'

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
