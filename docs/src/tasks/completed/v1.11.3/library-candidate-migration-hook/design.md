# Candidate API migration gate design

## Context

Tachyon の Lambda deploy は candidate Alias と Function URL を作成し、
`postDeploy` hookを実行した後にproduction Aliasを昇格する。このため
`postDeploy`はprovider上のcandidate作成後だが、production trafficから見れば
pre-release gateである。

汎用`preDeploy.command` JobRunはrepository checkoutやBuild artifactを持たない。
LibraryのRust/SQLx migrationをそこで直接実行することはできない。

## Proposed design

candidate Library API に `POST /internal/deploy/migrate` を追加する。endpointは
専用`LIBRARY_DEPLOY_HOOK_TOKEN`のbearer認証を必須とし、成功時だけ`200`を返す。token値、
DB credential、migration payloadはログへ出さない。

production manifestは次の順序で動作する。

1. Tachyonが新しいLibrary API candidateを作成する。
2. `postDeploy.command` JobRunへcandidate URLと既存secret referenceを渡す。
3. JobRunがcandidateのmigration endpointを呼ぶ。
4. candidateに組み込まれた新しいmigration setを実行する。
5. 成功時だけTachyonがproduction Aliasをcandidate Versionへ昇格する。

## Security

- endpointはpublic Function URL配下に存在するためbearer認証を必須とする。
- manifestにはsecret referenceだけを置き、値は保存しない。
- hook JobRunだけに専用tokenを注入し、commandやログには値を保存しない。
- 認証失敗は`401`、migration失敗は`500`とし、credentialをresponseへ含めない。
- migrationはSQLxの既存lock/idempotencyに従う。

## Alternatives

- 専用migration Lambdaの修正: runtimeとartifactを二重管理するため不採用。
- `preDeploy.command`でcargoを実行: source/artifactがないため実行不能。
- production API endpointでmigration:新しいmigration setを持たないため不採用。

## Rollout

1. API endpointとmanifestを同じcommitへ入れる。
2. production manifestをdry-run/applyする。
3. main Buildをtriggerし、candidate hookとAlias昇格を確認する。
4. authenticated browser smokeを実行する。
5. 旧migration Lambdaは参照がないことを確認後、別の破壊的cleanupとして扱う。
