---
name: GraphQLでProperty更新が成功し、削除後に一覧から消える
description: SELECT/MULTI_SELECTのoptions更新と削除後の消失確認を行う
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# GraphQLでProperty更新が成功し、削除後に一覧から消える

SELECT/MULTI_SELECTのoptions更新と削除後の消失確認を行う

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
          name: Org Update {{vars.timestamp}}
          username: org-update-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - org-update-{{vars.timestamp}}
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
      query: "mutation CreateRepo($input: CreateRepoInput!) {\n  createRepo(input: $input) { id username }\n}\n"
      variables:
        input:
          orgUsername: org-update-{{vars.timestamp}}
          repoName: Repo Update {{vars.timestamp}}
          repoUsername: repo-update-{{vars.timestamp}}
          userId: us_01hs2yepy5hw4rz8pdq2wywnwt
          isPublic: false
  expect:
    status: 200
    contains:
    - repo-update-{{vars.timestamp}}
```

## SELECTプロパティを追加する

```yaml scenario
steps:
- id: add_property_select
  name: SELECTプロパティを追加する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation AddProperty($input: PropertyInput!) {\n  addProperty(input: $input) { id name typ meta { ... on SelectType { options { key name } } } }\n}\n"
      variables:
        input:
          orgUsername: org-update-{{vars.timestamp}}
          repoUsername: repo-update-{{vars.timestamp}}
          propertyName: status
          propertyType: SELECT
          meta:
            select:
            - identifier: open
              label: Open
            - identifier: closed
              label: Closed
  expect:
    status: 200
    contains:
    - status
    - Open
  save:
    select_property_id: data.addProperty.id
```

## SELECTプロパティにoptionを追加する

```yaml scenario
steps:
- id: update_property_select_add_option
  name: SELECTプロパティにoptionを追加する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateProperty($id: String!, $input: PropertyInput!) {\n  updateProperty(id: $id, input: $input) { id name meta { ... on SelectType { options { key name } } } }\n}\n"
      variables:
        id: {{vars.select_property_id}}
        input:
          orgUsername: org-update-{{vars.timestamp}}
          repoUsername: repo-update-{{vars.timestamp}}
          propertyName: status
          propertyType: SELECT
          meta:
            select:
            - identifier: open
              label: Open
            - identifier: closed
              label: Closed
            - identifier: pending
              label: Pending
  expect:
    status: 200
    contains:
    - pending
```

## 更新後のプロパティ一覧に新オプションが含まれる

```yaml scenario
steps:
- id: properties_after_update
  name: 更新後のプロパティ一覧に新オプションが含まれる
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query Props($org: String!, $repo: String!) {\n  properties(orgUsername: $org, repoUsername: $repo) {\n    id\n    name\n    meta { ... on SelectType { options { key name } } }\n  }\n}\n"
      variables:
        org: org-update-{{vars.timestamp}}
        repo: repo-update-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - pending
```

## プロパティを削除する

```yaml scenario
steps:
- id: delete_property
  name: プロパティを削除する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation DeleteProperty($org: String!, $repo: String!, $id: String!) {\n  deleteProperty(orgUsername: $org, repoUsername: $repo, propertyId: $id)\n}\n"
      variables:
        org: org-update-{{vars.timestamp}}
        repo: repo-update-{{vars.timestamp}}
        id: {{vars.select_property_id}}
  expect:
    status: 200
    contains:
    - {{vars.select_property_id}}
```

## 削除後は一覧に含まれない

```yaml scenario
steps:
- id: properties_after_delete
  name: 削除後は一覧に含まれない
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query Props($org: String!, $repo: String!) {\n  properties(orgUsername: $org, repoUsername: $repo) { id name }\n}\n"
      variables:
        org: org-update-{{vars.timestamp}}
        repo: repo-update-{{vars.timestamp}}
  expect:
    status: 200
    not_contains:
    - {{vars.select_property_id}}
```
