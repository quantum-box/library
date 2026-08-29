# Library CLI 仕様

対象: Library の org / repo / data / property / source を端末と自動化から操作するための CLI (`library`)。

human が触る運用コマンドであると同時に、coding agent が Library を読み書きするための実行系でもある。後者を想定しているため、出力は `--json` で機械可読にでき、対話が必要な操作は非対話環境では黙って進まず失敗する。

## 1. 位置づけ

| | |
| --- | --- |
| package | `apps/cli` (`library-cli`) |
| バイナリ名 | `library` |
| 通信先 | library-api の REST / GraphQL / MCP endpoint |

CLI は library-api の public な HTTP interface だけを使う。DB へ直接触ることはなく、権限も API 側の policy check にそのまま従う。

ビルド:

```bash
cargo build -p library-cli --release
```

## 2. 認証

`pk_` で始まる Library API key を使う。key は Library client の API keys 画面で発行する。

解決順序は次のとおりで、先に見つかったものが勝つ。

1. `--api-key` / `--api-url` / `--operator-id` フラグ
2. 環境変数 `LIBRARY_API_KEY` / `LIBRARY_API_BASE_URL`
3. `library auth login` が保存したローカル profile

いずれも無い場合、API URL は `http://localhost:50055` にフォールバックする。

```bash
library auth login --api-key pk_xxx --api-url https://api.example.com
library auth status
library auth logout
```

`auth login` は保存前に key を API に問い合わせて検証する。検証を飛ばす場合は `--no-verify`。

profile の保存先:

| | |
| --- | --- |
| 既定 | `$XDG_CONFIG_HOME/library/config.json`、未設定時は `~/.config/library/config.json` |
| 上書き | 環境変数 `LIBRARY_CONFIG` にファイルパスを指定 |
| permission | `0600` |

`auth status` は key 全体を出さず prefix だけを表示する。

### CI / agent での指定

保存済み profile に依存させたくない場合は環境変数だけで完結する。

```bash
export LIBRARY_API_BASE_URL=https://api.example.com
export LIBRARY_API_KEY=pk_xxx
library --json repo list acme
```

## 3. グローバルオプション

| オプション | 用途 |
| --- | --- |
| `--api-url <URL>` | API base URL |
| `--api-key <KEY>` | API key |
| `--operator-id <ID>` | `x-operator-id` header。path に organization を含まない一部 endpoint 用 |
| `--json` | 表ではなく生の JSON を出す |

`--json` の出力は API の response をそのまま流す。表形式は人間向けの整形であり、列や幅は互換性を保証しない。**スクリプトと agent は必ず `--json` を使うこと。**

## 4. コマンド

repository を取る引数はすべて `org/repo` の形で指定する。

### `org`

| コマンド | 説明 |
| --- | --- |
| `library org get <username>` | organization と配下 repo を表示 |
| `library org create <username> [--name --description --website]` | organization を作成 |
| `library org update <username> --name <name> [--description --website]` | organization を更新 |

### `repo`

| コマンド | 説明 |
| --- | --- |
| `library repo list <org>` | organization の repo 一覧 |
| `library repo search [--org <org>] [--name <name>] [--limit <n>]` | repo を名前で検索。`--limit` の既定は 20 |
| `library repo get <org/repo>` | repo 詳細 |
| `library repo create <org/repo> [--name --description --public]` | repo を作成。`repo` 部分が username になる |
| `library repo update <org/repo> [--name --description --public --private --tag]` | repo 設定を更新。`--tag` は繰り返すと tag 一覧を置き換える |
| `library repo rename <org/repo> <new-username>` | repo の username を変更 |
| `library repo delete <org/repo> [--yes]` | repo と中身を削除 |

### `data`

| コマンド | 説明 |
| --- | --- |
| `library data list <org/repo> [--page --page-size]` | record 一覧。既定は `--page 1 --page-size 20` |
| `library data search <org/repo> <query> [--page --page-size]` | record を名前で検索 |
| `library data get <org/repo> <data-id>` | record 詳細 |
| `library data create <org/repo> --name <name> [--set ...]` | record を作成 |
| `library data update <org/repo> <data-id> --name <name> [--set ...]` | record を置換 |
| `library data delete <org/repo> <data-id> [--yes]` | record を削除 |

`data update` は PATCH ではなく置換である。指定しなかった property は空になるため、残したい値はすべて送り直す必要がある。

### `property`

| コマンド | 説明 |
| --- | --- |
| `library property list <org/repo>` | property 一覧 |
| `library property get <org/repo> <property-id>` | property 詳細 |
| `library property create <org/repo> <name> --type <type> [--auto-generate]` | property を作成 |
| `library property update <org/repo> <property-id> --name <name>` | property を rename |
| `library property delete <org/repo> <property-id> [--yes]` | property を削除 |

`--type` の値: `string`, `integer`, `html`, `markdown`, `relation`, `select`, `multi_select`, `id`, `location`, `date`, `image`, `rich_text`。`html` は API 側で `rich_text` に置き換えられた旧型。

`--auto-generate` は `--type id` でのみ必須で、他の型に付けると拒否される。

### `source`

| コマンド | 説明 |
| --- | --- |
| `library source list <org/repo>` | source 一覧 |
| `library source get <org/repo> <source-id>` | source 詳細 |
| `library source create <org/repo> <name> [--url]` | source を追加 |
| `library source update <org/repo> <source-id> [--name --url --clear-url]` | source を更新。`--clear-url` で URL を外す |
| `library source delete <org/repo> <source-id> [--yes]` | source を削除 |

### 削除の確認

`delete` は取り消せないため確認を求める。非対話環境 (CI / agent) には答える端末が無いので、prompt を黙って通すのではなく `--yes` が無ければ失敗する。

## 5. プロパティ値の指定

`data create` / `data update` は property を 3 種類のフラグで埋める。

| フラグ | 解釈 |
| --- | --- |
| `--set <PROPERTY>=<VALUE>` | プレーン文字列 |
| `--set-markdown <PROPERTY>=<VALUE>` | Markdown |
| `--set-json <PROPERTY>=<JSON>` | 生 JSON。数値・真偽値・配列・relation 用 |

`<PROPERTY>` には **property 名と property id のどちらでも書ける**。CLI が repo の property 一覧を引いて名前を id に解決し、どちらにも一致しなければ既知の property 名を添えて失敗する。

値は `@` 前置でファイルから読める。

| 書き方 | 意味 |
| --- | --- |
| `--set body=hello` | 文字列 `hello` |
| `--set-markdown body=@notes.md` | `notes.md` の中身 |
| `--set body=@-` | 標準入力 |
| `--set body=@@literal` | 先頭が `@` の文字列 `@literal` |

```bash
library data create acme/docs \
  --name 'Release notes' \
  --set-markdown body=@RELEASE.md \
  --set-json tags='["release","2026-08"]'
```

```bash
git log --oneline -20 | library data create acme/docs --name 'Recent commits' --set body=@-
```

## 6. `library mcp`

MCP server を client 無しで直接叩くためのサブコマンド。仕様は [MCP 連携仕様](mcp.md) を参照。

| コマンド | 説明 |
| --- | --- |
| `library mcp info` | `initialize` の結果を表示 |
| `library mcp tools` | 現在の credential で使える tool 一覧 |
| `library mcp call <name> [--arg --arg-json --arguments]` | tool を 1 つ呼ぶ |
| `library mcp config [--transport --name --no-key]` | MCP client 設定 JSON を出力 |

`mcp tools` は認証の有無で結果が変わる。key なしなら read tool だけ、key ありなら write tool も並ぶので、権限の確認に使える。

```bash
library mcp call list_data --arg org=acme --arg repo=docs --arg-json page_size=5
```

`--arg` も `@path` / `@-` でファイルと標準入力を読む。引数全体を JSON で渡す場合は `--arguments`。

`mcp config` は client の設定ファイルへ貼れる形を出す。`--transport` は `http` (既定) と `sse`。

`sse` は Non-GA で、server 側が `LIBRARY_MCP_SSE_ENABLED=true` を設定した環境でしか route が登録されない。現行の Lambda 配信では有効化できないため、通常は `http` を使う。`--transport sse` を指定すると stderr にその旨を出す。

```bash
library mcp config --transport sse --name library
```

```json
{
  "mcpServers": {
    "library": {
      "headers": {
        "Authorization": "Bearer pk_example"
      },
      "type": "sse",
      "url": "https://api.example.com/sse"
    }
  }
}
```

出力に key が含まれる場合は stderr に警告を出す。設定を共有したりログに残す場合は `--no-key` を使い、key は client 側で入れる。

## 7. coding agent から使う

agent には MCP と CLI の 2 経路がある。使い分け:

- **MCP**: agent が Library を「道具」として持ち、tool schema 経由で呼ぶ。tool 一覧が client に見えるため探索的な利用に向く。
- **CLI**: shell を持つ agent がパイプやファイルと組み合わせて使う。`@-` で他コマンドの出力をそのまま record に流せるのはこちらだけ。

CLI を agent に使わせる場合の前提:

1. `LIBRARY_API_BASE_URL` と `LIBRARY_API_KEY` を環境に置く (`auth login` に依存させない)
2. すべての呼び出しに `--json` を付ける
3. `delete` には `--yes` を付ける。付け忘れは prompt ではなくエラーになる

```bash
export LIBRARY_API_BASE_URL=https://api.example.com
export LIBRARY_API_KEY=pk_xxx

library --json repo list acme
library --json data list acme/docs --page-size 50
```

## 8. 既知の制約

- `library repo rename` は MCP tool には出していない（CLI / REST / GraphQL のみ）。認可の欠落は解消済みで、`PUT /v1beta/repos/{org}/{repo}/change-username` は `library:UpdateRepo` の resource-level チェックを通す（[apps/api/src/usecase/change_repo_username.rs](../../../apps/api/src/usecase/change_repo_username.rs)）。repo の owner / writer 以外は 403 になる。
- `data update` は置換であり、部分更新の口は無い。
- 表出力は互換性を保証しない。
- `mcp config --transport sse` が出す設定は、SSE を有効化した server にしか繋がらない。既定の配信では `http` を使う。詳細は [MCP 連携仕様](mcp.md) の GA status。

## 9. 実装参照

- CLI エントリポイント: [apps/cli/src/main.rs](../../../apps/cli/src/main.rs)
- 認証・profile: [apps/cli/src/config.rs](../../../apps/cli/src/config.rs)
- HTTP client: [apps/cli/src/client.rs](../../../apps/cli/src/client.rs)
- 出力整形: [apps/cli/src/output.rs](../../../apps/cli/src/output.rs)
- 各コマンド: [apps/cli/src/commands](../../../apps/cli/src/commands)
