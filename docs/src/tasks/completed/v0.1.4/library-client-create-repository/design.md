# Library Client repository作成・URL設計

## 問題

認証済みClientは組織とrepositoryを読み込める一方、repository作成は提供していない。特にrepositoryが0件の画面は説明だけで、次に取るべき操作がない。またoverview URLの`/repositories/{org}/{repo}`は、画面内で表示する`org/repo`表記と一致していない。

## 採用案

既存GraphQL `createRepo(input: CreateRepoInput!)`を利用する。Clientは認証済みuser IDと選択したorganizationのoperator IDを既存認証情報から付与し、作成成功後に一覧を再取得する。API responseが一覧へ反映されるまで遅延する場合に備え、作成結果をClient stateにも即時追加する。

作成ダイアログはorganization、repository名、slug、説明、公開範囲をまとめる。名前からslugを自動生成し、`organization / slug`をmonospaceのパスプレビューとして表示する。作成操作の結果として得られるURLを入力時点から示し、GitHub風のrepositoryモデルを視覚的にも一貫させる。

canonical routeは`/{organization}/{repository}`、settingsは`/{organization}/{repository}/settings`とする。既存`/repositories/{organization}/{repository}`とそのsettings URLはredirect専用routeとして残す。`/home`、`/databases`、`/docs`、`/chat`、`/sync`などの静的routeはroute tree上で引き続き優先される。

静的routeとの衝突を避けるため、Clientの組織作成では`home`、`organizations`、`repositories`、`databases`、`chat`、`sync`、`docs`、`documents`、`kanban`をorganization usernameの予約語とする。APIや他Clientにも同じ制約を適用することは後続のAPI側整合性課題とする。

## 導線

- サイドバーのRepositories見出し: 常に見える小さな`Create repository`操作。
- 組織overview: headerのprimary actionと0件empty state。
- All dataの0件dashboard: 次の操作として`Create repository`を表示。
- 作成成功: dialogを閉じ、canonical repository overviewへ移動。

## 代替案

- 旧Planet Libraryの作成画面へlinkする: Clientから離れ、認証と一覧更新が分断されるため不採用。
- `/repositories/{org}/{repo}`を維持する: 表示上の`org/repo`とURLが一致しないため不採用。
- repository作成専用pageを追加する: 入力が小さく、複数の導線から同じ操作を開始したいためdialogを採用。

## 認証・認可

既存access token、`x-platform-id`、選択organizationの`x-operator-id`を利用する。`userId`は保存済みLibrary auth identityから取得する。`library:CreateRepo` policy checkはAPI側に残し、Clientで権限を推測しない。

## 検証

- API client: mutation payload、organization header、auth user ID、成功response、GraphQL error。
- Context: 作成後の一覧更新、即時state追加、organization選択。
- UI: slug生成、organization選択、visibility、送信中、error表示。
- Router: canonical URL、settings、旧URL redirect、静的route回帰。
- Browser: Playwright fixtureでrepository作成からcanonical overview表示まで確認する。

## Rollout

Clientのみの変更として通常のCloud App buildで配布する。本番repository作成は別途、削除可能なテスト対象の承認後に行う。
