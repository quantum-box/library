---
name: REST APIでOrg/Repo/Property/Data/SourceがCRUDできる
description: RESTエンドポイントの主要ハッピーパスを一通り確認する
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# REST APIでOrg/Repo/Property/Data/SourceがCRUDできる

RESTエンドポイントの主要ハッピーパスを一通り確認する

## 組織を作成する

```yaml scenario
steps:
- id: create_org
  name: 組織を作成する
  request:
    method: POST
    url: /v1beta/orgs
    body:
      name: Org REST {{vars.timestamp}}
      username: org-rest-{{vars.timestamp}}
      description: rest org
  expect:
    status: 200
    contains:
    - org-rest-{{vars.timestamp}}
  save:
    org_username: username
```

## 組織詳細を取得する

```yaml scenario
steps:
- id: view_org
  name: 組織詳細を取得する
  request:
    method: GET
    url: /v1beta/orgs/{{vars.org_username}}
  expect:
    status: 200
    contains:
    - {{vars.org_username}}
```

## リポジトリを作成する

```yaml scenario
steps:
- id: create_repo
  name: リポジトリを作成する
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}
    body:
      name: Repo REST {{vars.timestamp}}
      username: repo-rest-{{vars.timestamp}}
      description: rest repo
      is_public: false
  expect:
    status: 200
    contains:
    - repo-rest-{{vars.timestamp}}
  save:
    repo_username: username
```

## リポジトリ詳細を取得する

```yaml scenario
steps:
- id: view_repo
  name: リポジトリ詳細を取得する
  request:
    method: GET
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}
  expect:
    status: 200
    contains:
    - {{vars.repo_username}}
```

## リポジトリを更新する

```yaml scenario
steps:
- id: update_repo
  name: リポジトリを更新する
  request:
    method: PUT
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}
    body:
      name: Repo REST updated {{vars.timestamp}}
      description: rest repo updated
      is_public: true
  expect:
    status: 200
    contains:
    - rest repo updated
```

## リポジトリのユーザー名を変更する

```yaml scenario
steps:
- id: change_repo_username
  name: リポジトリのユーザー名を変更する
  request:
    method: PUT
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/change-username
    body:
      new_username: repo-rest-new-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - repo-rest-new-{{vars.timestamp}}
  save:
    repo_username: username
```

## リポジトリ検索で新しいユーザー名が返る

```yaml scenario
steps:
- id: search_repo
  name: リポジトリ検索で新しいユーザー名が返る
  request:
    method: GET
    url: /v1beta/repos
    query:
      name: Repo REST updated {{vars.timestamp}}
      limit: 5
  expect:
    status: 200
    contains:
    - repo-rest-new-{{vars.timestamp}}
```

## STRINGプロパティを追加する

```yaml scenario
steps:
- id: add_property
  name: STRINGプロパティを追加する
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/properties
    body:
      name: title
      property_type: string
  expect:
    status: 200
    contains:
    - title
  save:
    property_id: id
```

## プロパティ一覧を取得する

```yaml scenario
steps:
- id: get_properties
  name: プロパティ一覧を取得する
  request:
    method: GET
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/properties
  expect:
    status: 200
    contains:
    - {{vars.property_id}}
```

## Dataを作成する

```yaml scenario
steps:
- id: add_data
  name: Dataを作成する
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/data
    body:
      name: First data REST {{vars.timestamp}}
      property_data:
      - property_id: {{vars.property_id}}
        value:
          string: hello
  expect:
    status: 200
    contains:
    - First data REST
  save:
    data_id: id
```

## Data詳細を取得する

```yaml scenario
steps:
- id: view_data
  name: Data詳細を取得する
  request:
    method: GET
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/data/{{vars.data_id}}
  expect:
    status: 200
    contains:
    - {{vars.data_id}}
```

## Dataを更新する

```yaml scenario
steps:
- id: update_data
  name: Dataを更新する
  request:
    method: PUT
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/data/{{vars.data_id}}
    body:
      name: First data REST updated {{vars.timestamp}}
      property_data:
      - property_id: {{vars.property_id}}
        value:
          string: hello-updated
  expect:
    status: 200
    contains:
    - hello-updated
```

## Data一覧を取得する

```yaml scenario
steps:
- id: data_list
  name: Data一覧を取得する
  request:
    method: GET
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/data-list
  expect:
    status: 200
    contains:
    - {{vars.data_id}}
```

## Data検索が成功する

```yaml scenario
steps:
- id: search_data
  name: Data検索が成功する
  request:
    method: GET
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/data
    query:
      name: First data REST {{vars.timestamp}}
      page: 1
      page_size: 20
  expect:
    status: 200
```

## プロパティを更新する

```yaml scenario
steps:
- id: update_property
  name: プロパティを更新する
  request:
    method: PUT
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/properties/{{vars.property_id}}
    body:
      name: title-updated
      property_type: string
  expect:
    status: 200
    contains:
    - title-updated
```

## Sourceを作成する

```yaml scenario
steps:
- id: create_source
  name: Sourceを作成する
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/sources
    body:
      name: src-rest-{{vars.timestamp}}
      url: https://example.com/rest/{{vars.timestamp}}
  expect:
    status: 201
    contains:
    - src-rest-{{vars.timestamp}}
  save:
    source_id: id
```

## Source一覧を取得する

```yaml scenario
steps:
- id: list_sources
  name: Source一覧を取得する
  request:
    method: GET
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/sources
  expect:
    status: 200
    contains:
    - {{vars.source_id}}
```

## Dataを削除する

```yaml scenario
steps:
- id: delete_data
  name: Dataを削除する
  request:
    method: DELETE
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/data/{{vars.data_id}}
  expect:
    status: 204
```

## Data削除後に取得で404になる

```yaml scenario
steps:
- id: get_data_after_delete
  name: Data削除後に取得で404になる
  request:
    method: GET
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/data/{{vars.data_id}}
  expect:
    status: 404
```

## プロパティを削除する

```yaml scenario
steps:
- id: delete_property
  name: プロパティを削除する
  request:
    method: DELETE
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/properties/{{vars.property_id}}
  expect:
    status: 200
```

## プロパティ削除後に取得で404になる

```yaml scenario
steps:
- id: get_property_after_delete
  name: プロパティ削除後に取得で404になる
  request:
    method: GET
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/properties/{{vars.property_id}}
  expect:
    status: 404
```

## 空nameのプロパティ追加は400になる

```yaml scenario
steps:
- id: add_property_invalid
  name: 空nameのプロパティ追加は400になる
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}/properties
    body:
      name: ''
      property_type: string
  expect:
    status: 200
```

## リポジトリを削除する

```yaml scenario
steps:
- id: delete_repo
  name: リポジトリを削除する
  request:
    method: DELETE
    url: /v1beta/repos/{{vars.org_username}}/{{vars.repo_username}}
  expect:
    status: 204
```
