# MCP ツール対応範囲

PLT-4345では、Claude Code / Codexから「所属先を探す → repositoryを選ぶ → schemaを確認する → Dataを読む・編集する → 結果を確認する」の操作を、Library CLI・REST・GraphQLの実装と比較した。

## 主要ワークフロー

| 操作 | MCP | 監査結果と対応 |
| --- | --- | --- |
| 現在の利用者 | `get_me` | 追加。認証済みuser / service accountのID・種別・名前を返す |
| 所属org一覧 | `list_orgs` | 追加。検証済みmembershipとLibraryのorganizationsテーブルの共通部分をページングする。API keyは発行orgだけ |
| org取得・作成・編集 | `get_org`, `create_org`, `update_org` | 既存。patchで省略した説明・URLを消さないよう修正。明示nullでクリア |
| repo一覧 | `list_repos` | 追加。既存のorg取得を使用しページング。非メンバーには公開repoのみ |
| repo検索・取得 | `search_repos`, `get_repo` | 既存。検索は所属org内の名前の部分一致。匿名検索の空結果を認証challengeに変更 |
| repo作成・編集・削除 | `create_repo`, `update_repo`, `delete_repo` | 既存。全write toolに認証challengeを適用 |
| repo slug変更 | `rename_repo` | CLI / REST / GraphQLにあった操作をMCPにも追加 |
| Data一覧・検索・取得 | `list_data`, `search_data`, `get_data` | 認証済みexecutorと実org IDを渡すよう修正。private repoの既存read権限を評価 |
| 型付きData読み取り | `get_data` | Markdownを維持したまま、`property_data`, `url`, `record_version` を追加。型付き編集に利用可能 |
| Data作成・編集・削除 | `create_data`, `update_data`, `delete_data` | 既存。write結果にもURL・型付き値・revisionを追加。`update_data`は指定Propertyだけをpatch |
| 指定IDへのData保存 | `upsert_data` | RESTのupsert usecaseを公開。ID再利用で重複作成を防ぐ。revision増加・同時更新は別問題 |
| Property CRUD | `list_properties`, `get_property`, `create_property`, `update_property`, `delete_property` | 既存。Dataの入力にも欠けていた`id` / `location`を追加 |
| Source CRUD | `list_sources`, `get_source`, `create_source`, `update_source`, `delete_source` | 既存。URL省略と明示nullが区別されず解除できなかった不具合を修正 |

合計29 tools。匿名時にもorg discovery用の`get_me` / `list_orgs`を広告するが、実行には認証が必要。write toolsは認証済み接続のカタログに追加される。カタログへの表示は操作権限の付与ではない。

## 認証と入力の契約

- org一覧は全tenantを走査しない。検証されたuser memberships / service account tenantのIDからLibraryのorgだけを取得する。他製品のtenantや重複membershipを混ぜない。
- `get_org` / `list_repos`で、単にログイン済みという理由だけで他orgのprivate repoを列挙しない。
- private Data取得は既存の`ViewDataList` / `SearchData` / `ViewData`の認可に従う。権限拒否と空データを区別する。
- 認証が必要なtool、未認証でのprivate read、無効なBearerには401とOAuth metadata challengeを返す。認証済みの権限拒否はJSON-RPCの`-32001`で返し、再ログインが解決策とは扱わない。
- MCP HTTPとSSEで検証済みcaller tokenを同じように引き継ぐ。API keyのpolicy評価も呼び出し元のcredentialを使う。内部SystemUserの処理は従来のservice credentialを維持する。
- `page`は1以上、`page_size`は1〜100。無効値は`-32602`。結果のないページは空配列と要求したページ番号を返す。
- `search_data`は現行のDB queryに合わせた **Data名の完全一致**。空queryなら一覧になる。以前のtool説明にあった「indexed content検索」は実装と一致しないため訂正した。全文検索やorg横断検索は提供しない。
- `upsert_data`は有効なData IDが必要。固定IDを再利用できるが、compare-and-swapや副作用を含めたexactly-once処理を提供するものではない。
- Property値は読み取りと書き込みで同じ`property_id` / `value_type` / `value`形式を使う。relationは対象Data IDの配列、locationは`latitude` / `longitude`、rich_textはJSON。自動生成Idは変更しない。
- nullableな文字列系Propertyは明示nullでクリアできる。booleanのnullもクリア、relationのnullは空配列、multi_selectのnullは選択解除。locationのnullクリアは現行usecaseにないため未対応であり、無効入力として拒否する。

## MCPに公開しない機能

全APIを自動的にtool化するのではなく、上記のナレッジ操作を今回の対応範囲とした。以下は既存UI / APIの専用フローを利用する。

| 機能 | 現在の入口と理由 |
| --- | --- |
| org/repoメンバー招待・権限・API key発行/失効 | UI / GraphQL / REST。アクセス管理・credentialの受け渡しは専用フローを維持 |
| Tachyon tenantのLibraryへの取り込み | UI / GraphQL。import可否評価とプロビジョニングを伴う |
| GitHub / Linear OAuth・import・sync設定 | UI / GraphQL / webhook。外部接続の設定・公開副作用を含む |
| 翻訳・公開言語・用語集・parquet export・画像upload | REST等の専用操作。課金やバイナリ転送を含み、通常のData CRUDとは別の契約 |
| Global ID mapping / MDM | GraphQL / REST。別のドメイン契約であり、今回の基本ナレッジ操作には含めない |
| org削除 | 既存Libraryの公開APIにも通常の削除操作がないため追加しない |

## 回帰検証

- `handler::mcp` の単体テスト: org membership境界、API keyの単一org制約、全write toolの認証指定、typed valueのround-trip、null/省略の区別、ページ境界。
- `sdk_auth::tests::context_calls_*`: user / service accountのcaller token引継ぎとSystemUserのcredential維持。
- `ga_crud_regression::mcp_authenticated_core_workflow_is_stable`: 実HTTP routerとテスト用Auth / MySQLで、org discoveryからprivate read、拒否、型付きpatch、Source URL解除、upsert再試行、rename、削除まで確認。

これらはローカル/CIの回帰検証。接続先へのデプロイ、各クライアントのOAuthログイン、実データでの確認は別途行う。
