---
name: Inbound Sync Webhook EndpointのCRUDが成功する
description: Webhook Endpointの作成・取得・更新・削除を検証する
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-operator-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# Inbound Sync Webhook EndpointのCRUDが成功する

Webhook Endpointの作成・取得・更新・削除を検証する

## GitHub Webhook Endpointを作成する

```yaml scenario
steps:
- id: create_github_endpoint
  name: GitHub Webhook Endpointを作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateWebhookEndpoint($input: CreateWebhookEndpointInput!) {\n  createWebhookEndpoint(input: $input) {\n    endpoint {\n      id\n      name\n      provider\n      status\n      events\n\
        \    }\n    webhookUrl\n    secret\n  }\n}\n"
      variables:
        input:
          name: GitHub Webhook {{vars.timestamp}}
          provider: GITHUB
          config: '{"provider":"github","repository":"test/repo","branch":"main","path_pattern":"docs/**/*.md"}'
          events:
          - push
          - pull_request
  expect:
    status: 200
    contains:
    - '"createWebhookEndpoint"'
    - GitHub Webhook {{vars.timestamp}}
    - GITHUB
    - webhookUrl
    - secret
  save:
    github_endpoint_id: data.createWebhookEndpoint.endpoint.id
    github_webhook_url: data.createWebhookEndpoint.webhookUrl
```

## GitHub Webhook Endpointを取得する

```yaml scenario
steps:
- id: get_github_endpoint
  name: GitHub Webhook Endpointを取得する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query GetWebhookEndpoint($id: String!) {\n  webhookEndpoint(id: $id) {\n    id\n    name\n    provider\n    status\n    events\n    config\n  }\n}\n"
      variables:
        id: {{steps.create_github_endpoint.outputs.github_endpoint_id}}
  expect:
    status: 200
    contains:
    - GitHub Webhook {{vars.timestamp}}
    - GITHUB
    - push
    - pull_request
```

## Linear Webhook Endpointを作成する

```yaml scenario
steps:
- id: create_linear_endpoint
  name: Linear Webhook Endpointを作成する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateWebhookEndpoint($input: CreateWebhookEndpointInput!) {\n  createWebhookEndpoint(input: $input) {\n    endpoint {\n      id\n      name\n      provider\n      status\n      events\n\
        \    }\n    webhookUrl\n    secret\n  }\n}\n"
      variables:
        input:
          name: Linear Webhook {{vars.timestamp}}
          provider: LINEAR
          config: '{"provider":"linear","team_id":null,"project_id":null}'
          events:
          - Issue
          - Project
  expect:
    status: 200
    contains:
    - '"createWebhookEndpoint"'
    - Linear Webhook {{vars.timestamp}}
    - LINEAR
  save:
    linear_endpoint_id: data.createWebhookEndpoint.endpoint.id
```

## テナントのWebhook Endpoint一覧を取得する

```yaml scenario
steps:
- id: list_endpoints
  name: テナントのWebhook Endpoint一覧を取得する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query ListWebhookEndpoints($tenantId: String!) {\n  webhookEndpoints(tenantId: $tenantId) {\n    id\n    name\n    provider\n    status\n  }\n}\n"
      variables:
        tenantId: tn_01j702qf86pc2j35s0kv0gv3gy
  expect:
    status: 200
    contains:
    - GitHub Webhook {{vars.timestamp}}
    - Linear Webhook {{vars.timestamp}}
```

## GitHub Endpoint のステータスを PAUSED に更新する

```yaml scenario
steps:
- id: update_status_paused
  name: GitHub Endpoint のステータスを PAUSED に更新する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateEndpointStatus($input: UpdateEndpointStatusInput!) {\n  updateWebhookEndpointStatus(input: $input) {\n    id\n    status\n  }\n}\n"
      variables:
        input:
          endpointId: {{steps.create_github_endpoint.outputs.github_endpoint_id}}
          status: PAUSED
  expect:
    status: 200
    contains:
    - PAUSED
```

## GitHub Endpoint のステータスを ACTIVE に戻す

```yaml scenario
steps:
- id: update_status_active
  name: GitHub Endpoint のステータスを ACTIVE に戻す
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateEndpointStatus($input: UpdateEndpointStatusInput!) {\n  updateWebhookEndpointStatus(input: $input) {\n    id\n    status\n  }\n}\n"
      variables:
        input:
          endpointId: {{steps.create_github_endpoint.outputs.github_endpoint_id}}
          status: ACTIVE
  expect:
    status: 200
    contains:
    - ACTIVE
```

## GitHub Endpoint のイベントを更新する

```yaml scenario
steps:
- id: update_events
  name: GitHub Endpoint のイベントを更新する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateEndpointEvents($input: UpdateEndpointEventsInput!) {\n  updateWebhookEndpointEvents(input: $input) {\n    id\n    events\n  }\n}\n"
      variables:
        input:
          endpointId: {{steps.create_github_endpoint.outputs.github_endpoint_id}}
          events:
          - push
          - release
  expect:
    status: 200
    contains:
    - push
    - release
```

## Linear Webhook Endpointを削除する

```yaml scenario
steps:
- id: delete_linear_endpoint
  name: Linear Webhook Endpointを削除する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation DeleteWebhookEndpoint($endpointId: String!) {\n  deleteWebhookEndpoint(endpointId: $endpointId)\n}\n"
      variables:
        endpointId: {{steps.create_linear_endpoint.outputs.linear_endpoint_id}}
  expect:
    status: 200
    contains:
    - '"deleteWebhookEndpoint":true'
```

## GitHub Webhook Endpointを削除する

```yaml scenario
steps:
- id: delete_github_endpoint
  name: GitHub Webhook Endpointを削除する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation DeleteWebhookEndpoint($endpointId: String!) {\n  deleteWebhookEndpoint(endpointId: $endpointId)\n}\n"
      variables:
        endpointId: {{steps.create_github_endpoint.outputs.github_endpoint_id}}
  expect:
    status: 200
    contains:
    - '"deleteWebhookEndpoint":true'
```

## 削除されたEndpointが取得できないことを確認

```yaml scenario
steps:
- id: verify_deletion
  name: 削除されたEndpointが取得できないことを確認
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "query ListWebhookEndpoints($tenantId: String!) {\n  webhookEndpoints(tenantId: $tenantId) {\n    id\n    name\n  }\n}\n"
      variables:
        tenantId: tn_01j702qf86pc2j35s0kv0gv3gy
  expect:
    status: 200
    not_contains:
    - GitHub Webhook {{vars.timestamp}}
    - Linear Webhook {{vars.timestamp}}
```
