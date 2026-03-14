---
name: Library Organizationの作成から更新までが成功する
description: CreateOrganization→UpdateOrganization（website含む）→Queryのハッピーパスを検証する
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# Library Organizationの作成から更新までが成功する

CreateOrganization→UpdateOrganization（website含む）→Queryのハッピーパスを検証する

## 組織を作成する

```yaml scenario
steps:
- id: create_org
  name: 組織を作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateOrg($input: CreateOrganizationInput!) {\n  createOrganization(input: $input) {\n    id\n    name\n    username\n    description\n    website\n  }\n}\n"
      variables:
        input:
          name: Org Lifecycle {{vars.timestamp}}
          username: org-lifecycle-{{vars.timestamp}}
          description: Organization for lifecycle test
          website: https://example.com
  expect:
    status: 200
    contains:
    - '"createOrganization"'
    - org-lifecycle-{{vars.timestamp}}
    - '"website":"https://example.com'
```

## 作成した組織を取得する

```yaml scenario
steps:
- id: get_org_initial
  name: 作成した組織を取得する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query GetOrg($username: String!) {\n  organization(username: $username) {\n    id\n    name\n    username\n    description\n    website\n  }\n}\n"
      variables:
        username: org-lifecycle-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - '"organization"'
    - org-lifecycle-{{vars.timestamp}}
    - '"website":"https://example.com'
    - Organization for lifecycle test
```

## 組織を更新する（website含む）

```yaml scenario
steps:
- id: update_org
  name: 組織を更新する（website含む）
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateOrg($input: UpdateOrganizationInput!) {\n  updateOrganization(input: $input) {\n    id\n    name\n    username\n    description\n    website\n  }\n}\n"
      variables:
        input:
          username: org-lifecycle-{{vars.timestamp}}
          name: Org Lifecycle Updated {{vars.timestamp}}
          description: Updated organization description
          website: https://updated-example.com
  expect:
    status: 200
    contains:
    - '"updateOrganization"'
    - Org Lifecycle Updated {{vars.timestamp}}
    - '"website":"https://updated-example.com'
    - Updated organization description
```

## 更新した組織を取得する

```yaml scenario
steps:
- id: get_org_updated
  name: 更新した組織を取得する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query GetOrg($username: String!) {\n  organization(username: $username) {\n    id\n    name\n    username\n    description\n    website\n  }\n}\n"
      variables:
        username: org-lifecycle-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - '"organization"'
    - Org Lifecycle Updated {{vars.timestamp}}
    - '"website":"https://updated-example.com'
    - Updated organization description
```

## 組織のwebsiteをnullに更新する

```yaml scenario
steps:
- id: update_org_remove_website
  name: 組織のwebsiteをnullに更新する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateOrg($input: UpdateOrganizationInput!) {\n  updateOrganization(input: $input) {\n    id\n    name\n    username\n    description\n    website\n  }\n}\n"
      variables:
        input:
          username: org-lifecycle-{{vars.timestamp}}
          name: Org Lifecycle Final {{vars.timestamp}}
          description: Final organization description
          website: null
  expect:
    status: 200
    contains:
    - '"updateOrganization"'
    - Org Lifecycle Final {{vars.timestamp}}
    - '"website":null'
```
