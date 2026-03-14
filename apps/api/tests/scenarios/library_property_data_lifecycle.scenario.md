---
name: Library RepoでのPropertyとDataのCRUDが成功する
description: Repo作成→Property追加→Data作成/一覧のハッピーパスを検証する
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# Library RepoでのPropertyとDataのCRUDが成功する

Repo作成→Property追加→Data作成/一覧のハッピーパスを検証する

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
          name: Org {{vars.timestamp}}p
          username: org-{{vars.timestamp}}p
          description: scenario org {{vars.timestamp}}p
  expect:
    status: 200
    contains:
    - '"createOrganization"'
    - org-{{vars.timestamp}}p
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
          orgUsername: org-{{vars.timestamp}}p
          repoName: Repo {{vars.timestamp}}p
          repoUsername: repo-{{vars.timestamp}}p
          userId: us_01hs2yepy5hw4rz8pdq2wywnwt
          isPublic: false
  expect:
    status: 200
    contains:
    - '"createRepo"'
    - repo-{{vars.timestamp}}p
```

## STRINGプロパティを追加する

```yaml scenario
steps:
- id: add_property_string
  name: STRINGプロパティを追加する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation AddProperty($input: PropertyInput!) {\n  addProperty(input: $input) { id name typ }\n}\n"
      variables:
        input:
          orgUsername: org-{{vars.timestamp}}p
          repoUsername: repo-{{vars.timestamp}}p
          propertyName: title
          propertyType: STRING
  expect:
    status: 200
    contains:
    - '"addProperty"'
    - title
  save:
    title_property_id: data.addProperty.id
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
          orgUsername: org-{{vars.timestamp}}p
          repoUsername: repo-{{vars.timestamp}}p
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
    status_property_id: data.addProperty.id
```

## MULTI_SELECTプロパティを追加する

```yaml scenario
steps:
- id: add_property_multi
  name: MULTI_SELECTプロパティを追加する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation AddProperty($input: PropertyInput!) {\n  addProperty(input: $input) { id name typ }\n}\n"
      variables:
        input:
          orgUsername: org-{{vars.timestamp}}p
          repoUsername: repo-{{vars.timestamp}}p
          propertyName: labels
          propertyType: MULTI_SELECT
          meta:
            multiSelect:
            - identifier: lab_a
              label: A
            - identifier: lab_b
              label: B
  expect:
    status: 200
    contains:
    - labels
  save:
    labels_property_id: data.addProperty.id
```

## Dataを作成する

```yaml scenario
steps:
- id: add_data
  name: Dataを作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation AddData($input: AddDataInputData!) {\n  addData(input: $input) {\n    id\n    name\n  }\n}\n"
      variables:
        input:
          actor: us_01hs2yepy5hw4rz8pdq2wywnwt
          orgUsername: org-{{vars.timestamp}}p
          repoUsername: repo-{{vars.timestamp}}p
          dataName: First data {{vars.timestamp}}
          propertyData:
          - propertyId: {{vars.title_property_id}}
            value:
              string: hello
  expect:
    status: 200
    contains:
    - '"name":"First data'
  save:
    data_id: data.addData.id
```

## Data一覧を取得し作成したデータが含まれることを確認

```yaml scenario
steps:
- id: data_list
  name: Data一覧を取得し作成したデータが含まれることを確認
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query DataList($org: String!, $repo: String!) {\n  dataList(orgUsername: $org, repoUsername: $repo, pageSize: 10, page: 1) {\n    items { id name }\n  }\n}\n"
      variables:
        org: org-{{vars.timestamp}}p
        repo: repo-{{vars.timestamp}}p
  expect:
    status: 200
    contains:
    - {{vars.data_id}}
```

## Dataを更新し値が変わることを確認

```yaml scenario
steps:
- id: update_data
  name: Dataを更新し値が変わることを確認
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateData($input: UpdateDataInputData!) {\n  updateData(input: $input) { id name propertyData { propertyId value { __typename ... on StringValue { string } } } }\n}\n"
      variables:
        input:
          actor: us_01hs2yepy5hw4rz8pdq2wywnwt
          orgUsername: org-{{vars.timestamp}}p
          repoUsername: repo-{{vars.timestamp}}p
          dataId: {{vars.data_id}}
          dataName: First data updated {{vars.timestamp}}
          propertyData:
          - propertyId: {{vars.title_property_id}}
            value:
              string: hello-updated
  expect:
    status: 200
    contains:
    - hello-updated
    - updateData
```

## MULTI_SELECTプロパティを削除する

```yaml scenario
steps:
- id: delete_property_multi
  name: MULTI_SELECTプロパティを削除する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation DeleteProperty($org: String!, $repo: String!, $id: String!) {\n  deleteProperty(orgUsername: $org, repoUsername: $repo, propertyId: $id)\n}\n"
      variables:
        org: org-{{vars.timestamp}}p
        repo: repo-{{vars.timestamp}}p
        id: {{vars.labels_property_id}}
  expect:
    status: 200
    contains:
    - {{vars.labels_property_id}}
```
