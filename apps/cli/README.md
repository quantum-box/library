# Library CLI

Library の org / repo / data / property / source を端末と自動化から操作する CLI です。バイナリ名は `library`。

human が触る運用コマンドであると同時に、coding agent が Library を読み書きするための実行系でもあります。後者を想定しているため、出力は `--json` で機械可読にでき、対話が必要な操作は非対話環境では黙って進まず失敗します。

通信先は library-api の REST / GraphQL / MCP endpoint です。DB へ直接触ることはなく、権限は API 側の policy check にそのまま従います。

## インストール

```bash
cargo build -p library-cli --release
```

`target/release/library` が生成されます。

## 認証

`pk_` で始まる Library API key を使います。key は Library client の API keys 画面で発行します。

```bash
library auth login --api-key pk_xxx --api-url https://library-api.txcloud.app
library auth status
library auth logout
```

`auth login` は保存前に key を API に問い合わせて検証します。飛ばす場合は `--no-verify`。`auth status` は key 全体ではなく prefix だけを表示します。

解決順序は次のとおりで、先に見つかったものが勝ちます。

1. `--api-key` / `--api-url` / `--operator-id` フラグ
2. 環境変数 `LIBRARY_API_KEY` / `LIBRARY_API_BASE_URL`
3. `library auth login` が保存したローカル profile

いずれも無い場合、API URL は `http://localhost:50055` にフォールバックします。

profile の保存先:

| | |
| --- | --- |
| 既定 | `$XDG_CONFIG_HOME/library/config.json`、未設定時は `~/.config/library/config.json` |
| 上書き | 環境変数 `LIBRARY_CONFIG` にファイルパスを指定 |
| permission | `0600` |

CI や agent からは、保存済み profile に依存させず環境変数だけで完結させます。

```bash
export LIBRARY_API_BASE_URL=https://library-api.txcloud.app
export LIBRARY_API_KEY=pk_xxx
library --json repo list acme
```

## コマンド

repository を取る引数はすべて `org/repo` の形で指定します。

| グループ | コマンド |
| --- | --- |
| `auth` | `login` `status` `logout` |
| `org` | `get` `create` `update` |
| `repo` | `list` `search` `get` `create` `update` `rename` `delete` |
| `data` | `list` `search` `get` `create` `update` `delete` |
| `property` | `list` `get` `create` `update` `delete` |
| `source` | `list` `get` `create` `update` `delete` |
| `mcp` | `info` `tools` `call` `config` |

各コマンドの引数は `library <group> <command> --help` で確認できます。

`--json` の出力は API の response をそのまま流します。表形式は人間向けの整形であり、列や幅は互換性を保証しません。**スクリプトと agent は必ず `--json` を使ってください。**

`data update` は PATCH ではなく置換です。指定しなかった property は空になるため、残したい値はすべて送り直す必要があります。

### 削除の確認

`delete` は取り消せないため確認を求めます。非対話環境 (CI / agent) には答える端末が無いので、prompt を黙って通すのではなく `--yes` が無ければ失敗します。

## プロパティ値の指定

`data create` / `data update` は property を 3 種類のフラグで埋めます。

| フラグ | 解釈 |
| --- | --- |
| `--set <PROPERTY>=<VALUE>` | プレーン文字列 |
| `--set-markdown <PROPERTY>=<VALUE>` | Markdown |
| `--set-json <PROPERTY>=<JSON>` | 生 JSON。数値・真偽値・配列・relation 用 |

`<PROPERTY>` には **property 名と property id のどちらでも書けます。** CLI が repo の property 一覧を引いて名前を id に解決し、どちらにも一致しなければ既知の property 名を添えて失敗します。

値は `@` 前置でファイルから読めます。

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

## MCP

`library mcp` は MCP server を client 無しで直接叩けます。

```bash
library mcp tools
library mcp call list_data --arg org=acme --arg repo=docs --arg-json page_size=5
```

`mcp tools` は認証の有無で結果が変わります。key なしなら read tool だけ、key ありなら write tool も並ぶので、権限の確認に使えます。

MCP client 用の設定はそのまま出力できます。

```bash
library mcp config --no-key
```

```json
{
  "mcpServers": {
    "library": {
      "type": "http",
      "url": "https://library-api.txcloud.app/mcp"
    }
  }
}
```

`--transport` は `http` (既定) と `sse` を取りますが、**`sse` は Non-GA で既定 off です。** server 側が `LIBRARY_MCP_SSE_ENABLED=true` を設定した環境でしか route が登録されないため、通常は `http` を使ってください。

## 開発

```bash
cargo test -p library-cli
cargo clippy -p library-cli --all-targets
cargo fmt -p library-cli
```

## 仕様

- [Library CLI 仕様](../../docs/specs/integrations/cli.md) — 全コマンドの引数、既知の制約
- [MCP 連携仕様](../../docs/specs/integrations/mcp.md) — tool 一覧、transport、GA status
