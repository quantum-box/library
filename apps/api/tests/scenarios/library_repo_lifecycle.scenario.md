---
name: Library Repoの作成から更新・名称変更までが成功する
description: CreateOrganization→CreateRepo→UpdateRepoのハッピーパスを検証する
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# Library Repoの作成から更新・名称変更までが成功する

CreateOrganization→CreateRepo→UpdateRepoのハッピーパスを検証する

## 組織を作成する

```yaml scenario
steps:
- id: create_org
  name: 組織を作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateOrg($input: CreateOrganizationInput!) {\n  createOrganization(input: $input) {\n    id\n    name\n    username\n  }\n}\n"
      variables:
        input:
          name: Org {{vars.timestamp}}
          username: org-{{vars.timestamp}}
          description: scenario org {{vars.timestamp}}
  expect:
    status: 200
    contains:
    - '"createOrganization"'
    - org-{{vars.timestamp}}
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
      query: "mutation CreateRepo($input: CreateRepoInput!) {\n  createRepo(input: $input) {\n    id\n    orgUsername\n    username\n    name\n    isPublic\n  }\n}\n"
      variables:
        input:
          orgUsername: org-{{vars.timestamp}}
          repoName: Repo {{vars.timestamp}}
          repoUsername: repo-{{vars.timestamp}}
          userId: us_01hs2yepy5hw4rz8pdq2wywnwt
          isPublic: false
          description: scenario repo {{vars.timestamp}}
  expect:
    status: 200
    contains:
    - '"createRepo"'
    - org-{{vars.timestamp}}
    - repo-{{vars.timestamp}}
    - '"isPublic":false'
```

## リポジトリを更新する

```yaml scenario
steps:
- id: update_repo
  name: リポジトリを更新する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n    description\n    isPublic\n    tags\n  }\n}\n"
      variables:
        input:
          orgUsername: org-{{vars.timestamp}}
          repoUsername: repo-{{vars.timestamp}}
          name: Repo Updated {{vars.timestamp}}
          description: updated desc {{vars.timestamp}}
          isPublic: true
          tags:
          - tag-a
          - tag-b
  expect:
    status: 200
    contains:
    - '"updateRepo"'
    - repo-{{vars.timestamp}}
    - '"isPublic":true'
    - '"tags":["tag-a","tag-b"]'
```
