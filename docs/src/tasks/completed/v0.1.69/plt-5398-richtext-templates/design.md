# PLT-5398 設計: リポジトリごとのRichTextテンプレート

## 背景

PLT-5398は、同じLibrary repoで共通の見出しや項目構成を持つレコードを繰り返し作る際、本文を手入力している問題を扱う。テンプレートは名前と書式付きRichText本文を持ち、repo編集者が管理し、新しいレコードにだけ任意で適用する。

Library APIはrepo情報とpolicyを `library` 管理DBに保存し、レコード本体はdatabase-manager側に置く。テンプレートはレコードデータではなくrepoの設定なので、`repos.id`を参照するrepo-scopedテーブルとして `library` 管理DBに保存する。database-manager内に置く案は、productionで両DBが分離されておりrepoとの外部キーでscopeを保証できないため採用しない。

## 採用設計

`repo_rich_text_templates` にテンプレートID、repo ID、名前、RichText JSON本文、作成・更新日時を保存する。repo IDを外部キーにし、repo削除時はテンプレートも削除する。全CRUDでorg/repoのusernameをrepo IDへ解決し、テンプレート検索・更新・削除の条件にそのIDを含める。repoのrenameで紐付けが変わらない。

APIは `richTextTemplates` queryとcreate/update/delete mutationを提供する。読み取りと変更は `library:UpdateRepo` のresource policy `trn:library:repo:{repo_id}` で保護する。利用者がReaderなら一覧・管理・新規レコードへの適用を許可しない。サーバー側で必ず検査し、UIの権限制御だけに依存しない。

テンプレート本文はRichTextのJSON文字列をそのまま保存・返却する。MarkdownやHTMLへ変換せず、BlockNoteが保持する空段落やブロック書式を壊さない。新規レコード作成では選択された本文をbody propertyへコピーし、その後は通常のrecord save経路を通す。テンプレート作成・更新はテンプレート行だけを書き換え、既存recordを検索・更新しない。

## UI

Library Web v1とPhoton v2の両方に、編集可能なrepo利用者向けのテンプレート一覧・作成・更新・削除を追加する。新規レコード作成時に選択欄を表示し、未選択なら従来どおり空の本文で作成できるようにする。本文編集には各UIの既存BlockNote editorを用いる。

issueの参考URLがWeb v1であること、Photon v2がRichText JSONを損失なく編集できることを踏まえて両UIを対象にした。両方でAPI、repo scope、権限、保存形式を共通化する。

## Migrationとrollback

新しいSQLx migrationでテーブルを追加し、通常のcandidate migration gateに適用させる。既存テーブルやrecord行は変更しない。アプリを旧版へ戻しても新テーブルは未参照のまま残るため、rollbackはコードを戻すだけとし、適用済みmigrationは本番でdown実行しない。

## 検証

- GraphQL query/mutationが編集可能な利用者だけに許可されること。
- 2つのrepoでテンプレートを作り、互いに一覧・変更・削除できないこと。
- 見出し・段落・箇条書きを含む本文がRichText JSONのまま新規recordへ反映されること。
- テンプレートを更新しても適用済み・未適用の既存recordが変わらないこと。
- テンプレートを選ばない作成経路が残ること。

## ADR

Libraryの既存repo-glossaryと同じ管理DB / repo外部キーの境界を使うfeature-localな設計であり、durableな横断ルールは追加しない。
