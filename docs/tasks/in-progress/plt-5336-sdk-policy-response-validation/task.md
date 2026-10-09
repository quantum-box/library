# PLT-5336: SDK認可応答を要求actionと照合する

- Issue: [PLT-5336](https://linear.app/quantum-box/issue/PLT-5336)
- 状態: 実装・最終検証中。2026-10-10 JSTの「全て直してマージして」に基づく公開・merge準備。
- Owning component: library-api。main `a5e33866e42b8853d0d81fe0c7844a4e8a8a3940` の1.11.9から1.11.10へpatch bump。

## 調査と修正

UIで利用できる非公開repoが本人認証MCPで403になる調査において、SDK経由のaction認可応答を検証する別の不具合を確認した。単一checkが空・別actionの応答を受けたり、一括checkが一部actionだけの応答を受けたりした場合に、要求したactionの完全な判定として扱う余地があった。

`apps/api/src/sdk_auth.rs` で単一checkは要求actionの判定1件、一括checkは要求actionと件数を含む完全な対応を確認する。missing/extra/mismatch、矛盾する重複、Allowとerrorの併存は固定error `Upstream protocol error` で拒否し、許可へ進めない。正しいAllow/Deny混在、一括要求での同じactionに対する同じ判定は維持する。通常のDenyと、上流transport/HTTP/decodeのerror分類は維持する。

resource checkは既存の必須booleanを使い、欠落・不正な値で拒否する回帰テストを追加する。resourceの認可契約、SDK依存、tenant隔離、user/serviceaccountの資格情報選択は変更しない。固定errorにtoken、任意上流応答、profile、private本文を含めない。

## 検証

- [x] mainのAPI versionとCargo.lockの対応entryを1.11.10に同期。
- [x] independent source security review: blocking finding 0件。
- [ ] 最終sourceに対する単一・一括応答とresource booleanの対象回帰、format/check。
- [ ] 公開PRのCI結果確認。
- [ ] merge結果確認。

最終テストとCIはこの記録更新時点では未完了であり、過去の別修正の成功を本変更の合格として扱わない。結果は公開PRの本文・報告へ追記する。client/desktopのversionやファイルは変更しない。

## 本番で未確認の点

相関済みの本番403はTachyonの代理caller gateで本人repo判定前に拒否されている。実runtime callerと判定時policy scopeは未確定であり、本変更のresponse検証不具合がその403の原因だとは断定しない。本番grant/IAM/credential変更、deploy、別資格情報によるprivate本文取得は行わず、同じ利用者のUI/MCP受入は未確認である。
