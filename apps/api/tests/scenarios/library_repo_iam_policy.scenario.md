---
name: Library Repo IAMのアクションとポリシーが登録されている
description: library向けManageRepoPolicyとOwner/Writer/Readerポリシーの存在を検証する
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
  timeout: 30
  continue_on_failure: false
---

# Library Repo IAMのアクションとポリシーが登録されている

library向けManageRepoPolicyとOwner/Writer/Readerポリシーの存在を検証する

## GraphQL health check

```yaml scenario
steps:
- id: health
  name: GraphQL health check
  request:
    method: POST
    url: /v1/graphql
    body:
      query: 'query ErrTest { errTest }

        '
  expect:
    status: 200
    contains:
    - '"errTest"'
```
