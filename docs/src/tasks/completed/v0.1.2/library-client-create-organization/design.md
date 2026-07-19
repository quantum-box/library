# Library Client 組織作成フロー設計

## 問題

組織切り替えはLibrary Client内で完結する一方、作成だけがPlanet Libraryに残っている。認証済みClientは既にLibrary GraphQLへaccess tokenとplatform headerを送れるため、同じ境界で既存mutationを呼び出せる。

## 採用案

サイドバーの組織switcherに隣接するボタンからmodalを開く。入力は表示名とusernameの2項目とし、usernameは小文字英数字とハイフンへ正規化する。送信時に `createOrganization(input: { name, username })` を呼び、成功後に組織一覧を再取得して作成した組織を選択する。

API clientはtransportとGraphQL errorを既存 `RecordApiError` に統一する。UIは送信中の多重実行を止め、失敗時にはmodalを閉じずAPI messageを表示する。

## 代替案

- Planet Libraryへlinkするだけ: 移動と再読込が必要なため不採用。
- 新しいREST endpointを追加する: 既存GraphQL mutationで要件を満たすため不採用。
- 組織とrepositoryを同時作成する: repository設定の入力と認可が別workflowになるため今回は扱わない。

## 認証・認可

既存access token、`x-platform-id`、`x-operator-id`の送信方法を再利用する。`library:CreateOrganization` policy checkはAPI側に残し、Clientで権限を推測しない。

## 検証

- API client: mutation payload、成功response、GraphQL error。
- Context: 作成後の一覧更新と選択。
- UI: validation、送信中、成功close、error表示。
- Browser: desktop/mobileの導線とfocus、既存切り替えの回帰。

## Rollout

Clientのpatch versionを上げて通常のCloud App buildで配布する。API・migration・環境変数変更はない。
