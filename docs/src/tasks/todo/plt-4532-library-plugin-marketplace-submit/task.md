# PLT-4532 Library plugin OpenAI提出運用

[Linear](https://linear.app/issue/PLT-4532) / [Library側実装](../../completed/v1.11.5/plt-4532-library-plugin-marketplace/task.md) / [提出資料](../../../../../plugins/library/submission/README.md)

Library側のReady PRとOpenAIへの最終提出を区別する。次のゲートを順に完了するまでSubmit for Reviewは実行しない。

- [ ] Ready PRをmergeし、Library API `1.11.5`とplugin package `0.5.1`を配布する
- [ ] portal発行tokenをrepository外の`OPENAI_APPS_CHALLENGE_TOKEN`へ設定してLibrary APIをデプロイする
- [ ] challenge endpointがHTTP 200、plain text、exact tokenだけを返すことを確認し、Verify Domainを成功させる
- [ ] Library向けprivacy policyとTerms of Serviceを法務・事業承認し、公開URLの実画面と内容を確認する
- [ ] MFA、SMS、email confirmation、社内ネットワーク不要の審査専用accountと架空fixtureを用意する
- [ ] OAuth UserInfoがdemo userの`email`と`email_verified: true`を返すことを確認する
- [ ] portalからOAuth authorizationし、Scan Toolsで29 tool、skills、annotationsを確認する
- [ ] ChatGPTとCodexでpositive 5件 / negative 3件を実行し、期待結果とデータ後始末を記録する
- [ ] Directory iconとcomposer iconをアップロードし、Developer Modeのdemo recording URLを設定する
- [ ] starter prompts、global availability、release notes、policy attestationsを最終確認する
- [ ] ユーザー確認後にSubmit for Reviewを実行する
