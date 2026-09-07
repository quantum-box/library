# Public repository docs UI prototype

PLT-4350 — Libraryのpublic repo向け閲覧専用ドキュメント表示案。

起動（リポジトリルートから）:

```sh
python3 -m http.server 4178 --bind 127.0.0.1 --directory prototypes/public-docs
```

http://127.0.0.1:4178 を開く。外部依存なし。

記事切り替え、記事メタデータ検索、ページ内目次、前後の記事、モバイルメニューを実装。サンプルは画面検討専用で、製品の実操作を保証するマニュアルではない。API接続・認可・SSR・公開設定は含まない。既存 `/public` ルートは変更していない。

ブラウザ確認: デスクトップ表示、検索「予約」、記事切り替え、390px幅での表示とメニューからの記事移動。
