# Initiative・Project のオンライン表示

リポジトリの **戦略・実行** タブ (`/{organization}/{repository}/strategy`) で、
Tachyonから受け入れたInitiative・Projectを表示する。通常のData一覧とは別の読取専用画面。

- 全件／Initiative／Projectの絞り込み、名前検索、50件ごとの追加取得。
- 同じProjectは一行で保持。詳細から複数のInitiativeとの関係を確認・移動できる。
- 元の名称・状態・担当・期限は編集不可。Library所有のメモ・関連文書だけ保存できる。
- 元の更新日時、Libraryの最終取得日時、画面の確認日時を分けて表示する。
- 同期途中・追加ページ・未解決の関係を明示し、全件取得済みと誤認させない。
- リクエストには既存の認証・operatorヘッダーと`cache: no-store`を使用する。
- Photon/Yjs、IndexedDB、localStorage、sessionStorageへ投影を書き込まない。
  オフライン時・タブを非表示にした時・閲覧権確認が失敗した時に表示を消去する。
- 公開／共有ルートは追加しない。このタブでは公開文書アクションを表示しない。

## API

`GET /v1beta/repos/{org}/{repo}/pm-projection?after=&limit=50` と
`GET .../pm-projection/records/{id}` を使う。
メモ保存は`POST .../pm-projection/{id}/context`へ`{note,document_ids}`を送る。
関連文書には同じリポジトリの文書リンク、または文書IDを入力できる。保存時のアクセス権は
サーバーが検証する。同期元の変更や接続設定をブラウザから直接操作しない。

利用にはLibrary側の`LIBRARY_STRATEGY_ENABLED=true`と、管理者による非公開リポジトリへの
subscription接続・初期snapshot・writer有効化が必要。

## 検証

- `npm test -- src/lib/strategyProjectionApi.test.ts src/components/RepositoryStrategyView.test.tsx src/i18n/catalogs.test.ts`
- `npm run type-check`
- `npx playwright test tests/e2e/strategy.spec.ts --project=chromium`

ブラウザテストは既存のローカルAPI fixtureとページ限定のPM応答fixtureを利用する。
ナビゲーション、共有関係、メモ保存、閲覧取消、部分取得を確認する。本番認証や実際のLinearとの
接続を検証したものではない。

2026-10-24 検証結果:

- APIクライアント・画面・全言語カタログ: 55件成功（3ファイル）。
- TypeScript型チェック、変更した実装・テストのESLint、production build成功。
- Chromiumブラウザテスト: 2件成功。リポジトリナビゲーションからの到達、
  2つのInitiativeと共有するProject、ローカルメモ保存、403時のデータ消去、追加取得を確認。
- `test-results/strategy-project-details.png` と `test-results/strategy-revoked.png`
  を目視確認。これらの生成物はGit管理対象外。

認可連携の前提として、LibraryサーバーからTachyonへの
`POST /v1beta/{tenant}/pm-sync/subscriptions/{id}/validate-resources` が必要。
このサーバー間の実認可確認は上記の画面fixtureテストでは代替できない。

期限（target_date）はUTCの暦日として日付だけを表示し、取得・更新日時は時刻を表示する。
追加画面テスト1件成功（画面テスト計6件）、ESLint成功。
