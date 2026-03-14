---
name: Public/Privateリポジトリのアクセス制御が正しく動作する
description: 'Public/Privateリポジトリへの認証あり・なしアクセスを検証する。

  - Publicリポジトリは認証なしでGraphQL repoクエリ（policies含む）が取得可能

  - Privateリポジトリは認証なしでアクセスするとPermissionDenied

  - 存在しないリポジトリへのアクセスはNotFound

  '
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-operator-id: tn_01hjryxysgey07h5jz5wagqj0m
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# Public/Privateリポジトリのアクセス制御が正しく動作する

Public/Privateリポジトリへの認証あり・なしアクセスを検証する。
- Publicリポジトリは認証なしでGraphQL repoクエリ（policies含む）が取得可能
- Privateリポジトリは認証なしでアクセスするとPermissionDenied
- 存在しないリポジトリへのアクセスはNotFound


## テスト用組織を作成する

```yaml scenario
steps:
- id: create_org
  name: テスト用組織を作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateOrg($input: CreateOrganizationInput!) {\n  createOrganization(input: $input) {\n    id\n    username\n  }\n}\n"
      variables:
        input:
          name: Visibility Test Org {{vars.timestamp}}
          username: vis-test-{{vars.timestamp}}
          description: Organization for repo visibility test
  expect:
    status: 200
    contains:
    - '"createOrganization"'
    - vis-test-{{vars.timestamp}}
```

## Publicリポジトリを作成する

```yaml scenario
steps:
- id: create_public_repo
  name: Publicリポジトリを作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateRepo($input: CreateRepoInput!) {\n  createRepo(input: $input) {\n    id\n    username\n    isPublic\n  }\n}\n"
      variables:
        input:
          orgUsername: vis-test-{{vars.timestamp}}
          repoName: Public Repo {{vars.timestamp}}
          repoUsername: pub-repo-{{vars.timestamp}}
          userId: us_01hs2yepy5hw4rz8pdq2wywnwt
          isPublic: true
          description: Public repo for visibility test
  expect:
    status: 200
    contains:
    - '"createRepo"'
    - pub-repo-{{vars.timestamp}}
    - '"isPublic":true'
```

## Privateリポジトリを作成する

```yaml scenario
steps:
- id: create_private_repo
  name: Privateリポジトリを作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateRepo($input: CreateRepoInput!) {\n  createRepo(input: $input) {\n    id\n    username\n    isPublic\n  }\n}\n"
      variables:
        input:
          orgUsername: vis-test-{{vars.timestamp}}
          repoName: Private Repo {{vars.timestamp}}
          repoUsername: priv-repo-{{vars.timestamp}}
          userId: us_01hs2yepy5hw4rz8pdq2wywnwt
          isPublic: false
          description: Private repo for visibility test
  expect:
    status: 200
    contains:
    - '"createRepo"'
    - priv-repo-{{vars.timestamp}}
    - '"isPublic":false'
```

## 認証ありでPublicリポジトリのrepoクエリが成功する

```yaml scenario
steps:
- id: auth_view_public_repo
  name: 認証ありでPublicリポジトリのrepoクエリが成功する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query ViewRepo($org: String!, $repo: String!) {\n  repo(orgUsername: $org, repoUsername: $repo) {\n    id\n    name\n    username\n    isPublic\n    description\n    policies {\n      userId\n\
        \      role\n      user {\n        id\n        username\n        name\n      }\n    }\n    sources {\n      id\n      name\n    }\n  }\n}\n"
      variables:
        org: vis-test-{{vars.timestamp}}
        repo: pub-repo-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - '"repo"'
    - Public Repo {{vars.timestamp}}
    - '"isPublic":true'
    - '"policies"'
    - '"role"'
```

## 認証なしでPublicリポジトリのrepoクエリが成功する

```yaml scenario
steps:
- id: anon_view_public_repo
  name: 認証なしでPublicリポジトリのrepoクエリが成功する
  request:
    method: POST
    url: /v1/graphql
    headers:
      Authorization: ''
    body:
      query: "query ViewRepo($org: String!, $repo: String!) {\n  repo(orgUsername: $org, repoUsername: $repo) {\n    id\n    name\n    username\n    isPublic\n    description\n    policies {\n      userId\n\
        \      role\n    }\n    sources {\n      id\n      name\n    }\n  }\n}\n"
      variables:
        org: vis-test-{{vars.timestamp}}
        repo: pub-repo-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - '"repo"'
    - Public Repo {{vars.timestamp}}
    - '"isPublic":true'
    - '"policies"'
```

## 認証なしでPrivateリポジトリにアクセスするとPermissionDenied

```yaml scenario
steps:
- id: anon_view_private_repo
  name: 認証なしでPrivateリポジトリにアクセスするとPermissionDenied
  request:
    method: POST
    url: /v1/graphql
    headers:
      Authorization: ''
    body:
      query: "query ViewRepo($org: String!, $repo: String!) {\n  repo(orgUsername: $org, repoUsername: $repo) {\n    id\n    name\n    isPublic\n  }\n}\n"
      variables:
        org: vis-test-{{vars.timestamp}}
        repo: priv-repo-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - errors
    - PermissionDenied
```

## 存在しないリポジトリにアクセスするとNotFound

```yaml scenario
steps:
- id: anon_view_nonexistent_repo
  name: 存在しないリポジトリにアクセスするとNotFound
  request:
    method: POST
    url: /v1/graphql
    headers:
      Authorization: ''
    body:
      query: "query ViewRepo($org: String!, $repo: String!) {\n  repo(orgUsername: $org, repoUsername: $repo) {\n    id\n    name\n  }\n}\n"
      variables:
        org: vis-test-{{vars.timestamp}}
        repo: nonexistent-repo-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - errors
    - NotFoundError
```

## 認証ありの非メンバーがPrivateリポジトリにアクセスするとPermissionDenied

```yaml scenario
steps:
- id: non_member_view_private_repo
  name: 認証ありの非メンバーがPrivateリポジトリにアクセスするとPermissionDenied
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01ke1h5471vxsbscp8jd3bramn
    body:
      query: "query ViewRepo($org: String!, $repo: String!) {\n  repo(orgUsername: $org, repoUsername: $repo) {\n    id\n    name\n    isPublic\n  }\n}\n"
      variables:
        org: vis-test-{{vars.timestamp}}
        repo: priv-repo-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - errors
    - PermissionDenied
```

## 認証ありの非メンバーでもPublicリポジトリは閲覧可能

```yaml scenario
steps:
- id: non_member_view_public_repo
  name: 認証ありの非メンバーでもPublicリポジトリは閲覧可能
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01ke1h5471vxsbscp8jd3bramn
    body:
      query: "query ViewRepo($org: String!, $repo: String!) {\n  repo(orgUsername: $org, repoUsername: $repo) {\n    id\n    name\n    isPublic\n    policies {\n      userId\n      role\n      user {\n\
        \        id\n        username\n      }\n    }\n  }\n}\n"
      variables:
        org: vis-test-{{vars.timestamp}}
        repo: pub-repo-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - '"repo"'
    - Public Repo {{vars.timestamp}}
    - '"isPublic":true'
    - '"policies"'
```

## 存在しない組織のリポジトリにアクセスするとNotFound

```yaml scenario
steps:
- id: anon_view_nonexistent_org
  name: 存在しない組織のリポジトリにアクセスするとNotFound
  request:
    method: POST
    url: /v1/graphql
    headers:
      Authorization: ''
    body:
      query: "query ViewRepo($org: String!, $repo: String!) {\n  repo(orgUsername: $org, repoUsername: $repo) {\n    id\n    name\n  }\n}\n"
      variables:
        org: nonexistent-org-{{vars.timestamp}}
        repo: some-repo
  expect:
    status: 200
    contains:
    - errors
    - NotFoundError
```
