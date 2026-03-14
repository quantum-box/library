---
name: Library SourceとAPI KeyのCRUDが成功する
description: Org/Repo作成後にSourceの作成・取得・更新・削除とAPI Key作成/一覧を検証する
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# Library SourceとAPI KeyのCRUDが成功する

Org/Repo作成後にSourceの作成・取得・更新・削除とAPI Key作成/一覧を検証する

## 組織を作成する

```yaml scenario
steps:
- id: create_org
  name: 組織を作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateOrg($input: CreateOrganizationInput!) {\n  createOrganization(input: $input) { id username }\n}\n"
      variables:
        input:
          name: Org {{vars.timestamp}}s
          username: org-{{vars.timestamp}}s
          description: scenario org {{vars.timestamp}}s
  expect:
    status: 200
    contains:
    - '"createOrganization"'
    - org-{{vars.timestamp}}s
```

## リポジトリを作成する

```yaml scenario
steps:
- id: create_repo
  name: リポジトリを作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateRepo($input: CreateRepoInput!) {\n  createRepo(input: $input) { id username orgUsername }\n}\n"
      variables:
        input:
          orgUsername: org-{{vars.timestamp}}s
          repoName: Repo {{vars.timestamp}}s
          repoUsername: repo-{{vars.timestamp}}s
          userId: us_01hs2yepy5hw4rz8pdq2wywnwt
          isPublic: true
  expect:
    status: 200
    contains:
    - repo-{{vars.timestamp}}s
```

## Sourceを作成する

```yaml scenario
steps:
- id: create_source
  name: Sourceを作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateSource($input: CreateSourceInput!) {\n  createSource(input: $input) { id name url }\n}\n"
      variables:
        input:
          orgUsername: org-{{vars.timestamp}}s
          repoUsername: repo-{{vars.timestamp}}s
          name: Source {{vars.timestamp}}
          url: https://example.com/{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - '"createSource"'
    - Source {{vars.timestamp}}
  save:
    source_id: data.createSource.id
```

## Sourceを取得する

```yaml scenario
steps:
- id: get_source
  name: Sourceを取得する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query GetSource($org: String!, $repo: String!, $id: String!) {\n  source(orgUsername: $org, repoUsername: $repo, sourceId: $id) {\n    id\n    name\n    url\n  }\n}\n"
      variables:
        org: org-{{vars.timestamp}}s
        repo: repo-{{vars.timestamp}}s
        id: {{vars.source_id}}
  expect:
    status: 200
    contains:
    - {{vars.source_id}}
```

## Sourceを更新する

```yaml scenario
steps:
- id: update_source
  name: Sourceを更新する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateSource($input: UpdateSourceInput!) {\n  updateSource(input: $input) { id name url }\n}\n"
      variables:
        input:
          orgUsername: org-{{vars.timestamp}}s
          repoUsername: repo-{{vars.timestamp}}s
          sourceId: {{vars.source_id}}
          name: Source Updated {{vars.timestamp}}
          url: https://example.com/updated/{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - Source Updated {{vars.timestamp}}
```

## Sourceを削除する

```yaml scenario
steps:
- id: delete_source
  name: Sourceを削除する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation DeleteSource($org: String!, $repo: String!, $id: String!) {\n  deleteSource(orgUsername: $org, repoUsername: $repo, sourceId: $id)\n}\n"
      variables:
        org: org-{{vars.timestamp}}s
        repo: repo-{{vars.timestamp}}s
        id: {{vars.source_id}}
  expect:
    status: 200
    contains:
    - {{vars.source_id}}
```

## API Keyを作成する

```yaml scenario
steps:
- id: create_api_key
  name: API Keyを作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateApiKey($input: CreateApiKeyInput!) {\n  createApiKey(input: $input) {\n    apiKey { id name value }\n    serviceAccount { id name }\n  }\n}\n"
      variables:
        input:
          organizationUsername: org-{{vars.timestamp}}s
          name: key-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - key-{{vars.timestamp}}
  save:
    api_key_id: data.createApiKey.apiKey.id
```

## API Key一覧に作成したキーが含まれることを確認

```yaml scenario
steps:
- id: list_api_keys
  name: API Key一覧に作成したキーが含まれることを確認
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query ApiKeys($org: String!) {\n  apiKeys(orgUsername: $org) { id name }\n}\n"
      variables:
        org: org-{{vars.timestamp}}s
  expect:
    status: 200
    contains:
    - key-{{vars.timestamp}}
```
