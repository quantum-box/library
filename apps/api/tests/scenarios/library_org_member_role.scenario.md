---
name: 組織メンバーのロール変更とリポジトリアクセス権限が正しく動作する
description: 'changeOrgMemberRole mutationでオーナー昇格/降格時にロールが正しく更新され、

  OWNERは全リポジトリにアクセスできるが、GENERALは権限なしでアクセスできないことを検証する

  '
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-operator-id: tn_01hjryxysgey07h5jz5wagqj0m
    x-user-id: us_01ke1h5471vxsbscp8jd3bramn
  timeout: 30
  continue_on_failure: false
---

# 組織メンバーのロール変更とリポジトリアクセス権限が正しく動作する

changeOrgMemberRole mutationでオーナー昇格/降格時にロールが正しく更新され、
OWNERは全リポジトリにアクセスできるが、GENERALは権限なしでアクセスできないことを検証する


## テスト開始時にOWNERロールであることを確認する

```yaml scenario
steps:
- id: ensure_owner_role
  name: テスト開始時にOWNERロールであることを確認する
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
    body:
      query: "mutation ChangeRole($input: ChangeOrgMemberRoleInput!) {\n  changeOrgMemberRole(input: $input) {\n    id\n    role\n  }\n}\n"
      variables:
        input:
          tenantId: tn_01hjryxysgey07h5jz5wagqj0m
          userId: us_01ke1h5471vxsbscp8jd3bramn
          newRole: OWNER
  expect:
    status: 200
    contains:
    - '"role":"OWNER"'
```

## テスト用組織を作成する

```yaml scenario
steps:
- id: create_org
  name: テスト用組織を作成する
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
    body:
      query: "mutation CreateOrg($input: CreateOrganizationInput!) {\n  createOrganization(input: $input) {\n    id\n    username\n  }\n}\n"
      variables:
        input:
          name: Role Test Org {{vars.timestamp}}
          username: role-test-org-{{vars.timestamp}}
          description: Organization for role access test
  expect:
    status: 200
    contains:
    - '"createOrganization"'
    - role-test-org-{{vars.timestamp}}
```

## プライベートリポジトリを作成する

```yaml scenario
steps:
- id: create_private_repo
  name: プライベートリポジトリを作成する
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
    body:
      query: "mutation CreateRepo($input: CreateRepoInput!) {\n  createRepo(input: $input) {\n    id\n    username\n    isPublic\n  }\n}\n"
      variables:
        input:
          orgUsername: role-test-org-{{vars.timestamp}}
          repoName: Private Repo {{vars.timestamp}}
          repoUsername: private-repo-{{vars.timestamp}}
          userId: us_01hs2yepy5hw4rz8pdq2wywnwt
          isPublic: false
          description: Private repo for access test
  expect:
    status: 200
    contains:
    - '"createRepo"'
    - private-repo-{{vars.timestamp}}
    - '"isPublic":false'
```

## OWNERはリポジトリを更新できる

```yaml scenario
steps:
- id: owner_can_update_repo
  name: OWNERはリポジトリを更新できる
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n    description\n  }\n}\n"
      variables:
        input:
          orgUsername: role-test-org-{{vars.timestamp}}
          repoUsername: private-repo-{{vars.timestamp}}
          name: Updated by Owner
          description: Updated by OWNER role
  expect:
    status: 200
    contains:
    - '"updateRepo"'
    - Updated by Owner
```

## ユーザーをGENERALロールに降格する（repoOwnerポリシーが剥奪される）

```yaml scenario
steps:
- id: change_to_general
  name: ユーザーをGENERALロールに降格する（repoOwnerポリシーが剥奪される）
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
    body:
      query: "mutation ChangeRole($input: ChangeOrgMemberRoleInput!) {\n  changeOrgMemberRole(input: $input) {\n    id\n    role\n  }\n}\n"
      variables:
        input:
          tenantId: tn_01hjryxysgey07h5jz5wagqj0m
          userId: us_01ke1h5471vxsbscp8jd3bramn
          newRole: GENERAL
  expect:
    status: 200
    contains:
    - '"role":"GENERAL"'
```

## GENERALはリポジトリを更新できない（権限エラー）

```yaml scenario
steps:
- id: general_cannot_update_repo
  name: GENERALはリポジトリを更新できない（権限エラー）
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n  }\n}\n"
      variables:
        input:
          orgUsername: role-test-org-{{vars.timestamp}}
          repoUsername: private-repo-{{vars.timestamp}}
          name: Should Fail
          description: This update should fail
  expect:
    status: 200
    contains:
    - errors
```

## ユーザーをOWNERロールに昇格する（repoOwnerポリシーが付与される）

```yaml scenario
steps:
- id: upgrade_to_owner
  name: ユーザーをOWNERロールに昇格する（repoOwnerポリシーが付与される）
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
    body:
      query: "mutation ChangeRole($input: ChangeOrgMemberRoleInput!) {\n  changeOrgMemberRole(input: $input) {\n    id\n    role\n  }\n}\n"
      variables:
        input:
          tenantId: tn_01hjryxysgey07h5jz5wagqj0m
          userId: us_01ke1h5471vxsbscp8jd3bramn
          newRole: OWNER
  expect:
    status: 200
    contains:
    - '"role":"OWNER"'
```

## OWNERに戻るとリポジトリを再び更新できる

```yaml scenario
steps:
- id: owner_can_update_repo_again
  name: OWNERに戻るとリポジトリを再び更新できる
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n    description\n  }\n}\n"
      variables:
        input:
          orgUsername: role-test-org-{{vars.timestamp}}
          repoUsername: private-repo-{{vars.timestamp}}
          name: Updated by Owner Again
          description: Updated after role upgrade
  expect:
    status: 200
    contains:
    - '"updateRepo"'
    - Updated by Owner Again
```

## リポジトリ権限テストのためGENERALに降格する

```yaml scenario
steps:
- id: change_to_general_for_repo_permission_test
  name: リポジトリ権限テストのためGENERALに降格する
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
    body:
      query: "mutation ChangeRole($input: ChangeOrgMemberRoleInput!) {\n  changeOrgMemberRole(input: $input) {\n    id\n    role\n  }\n}\n"
      variables:
        input:
          tenantId: tn_01hjryxysgey07h5jz5wagqj0m
          userId: us_01ke1h5471vxsbscp8jd3bramn
          newRole: GENERAL
  expect:
    status: 200
    contains:
    - '"role":"GENERAL"'
```

## GENERALは招待前はリポジトリを更新できない

```yaml scenario
steps:
- id: general_cannot_update_repo_before_invite
  name: GENERALは招待前はリポジトリを更新できない
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n  }\n}\n"
      variables:
        input:
          orgUsername: role-test-org-{{vars.timestamp}}
          repoUsername: private-repo-{{vars.timestamp}}
          name: Should Fail Before Invite
          description: This update should fail
  expect:
    status: 200
    contains:
    - errors
```

## GENERALユーザーをリポジトリメンバーとして招待する（writer権限）

```yaml scenario
steps:
- id: invite_general_user_to_repo
  name: GENERALユーザーをリポジトリメンバーとして招待する（writer権限）
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
    body:
      query: "mutation InviteRepoMember($input: InviteRepoMemberInput!) {\n  inviteRepoMember(input: $input)\n}\n"
      variables:
        input:
          orgUsername: role-test-org-{{vars.timestamp}}
          repoUsername: private-repo-{{vars.timestamp}}
          repoId: {{steps.create_private_repo.outputs.createRepo.id}}
          usernameOrEmail: test2
          role: writer
  expect:
    status: 200
    contains:
    - '"inviteRepoMember":true'
```

## 招待後はGENERALでもリポジトリを更新できる

```yaml scenario
steps:
- id: general_can_update_repo_after_invite
  name: 招待後はGENERALでもリポジトリを更新できる
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n    description\n  }\n}\n"
      variables:
        input:
          orgUsername: role-test-org-{{vars.timestamp}}
          repoUsername: private-repo-{{vars.timestamp}}
          name: Updated by GENERAL after invite
          description: Updated after repo-level permission grant
  expect:
    status: 200
    contains:
    - '"updateRepo"'
    - Updated by GENERAL after invite
```

## 無効なロールを指定するとエラーになる

```yaml scenario
steps:
- id: invalid_role_error
  name: 無効なロールを指定するとエラーになる
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
    body:
      query: "mutation ChangeRole($input: ChangeOrgMemberRoleInput!) {\n  changeOrgMemberRole(input: $input) {\n    id\n    role\n  }\n}\n"
      variables:
        input:
          tenantId: tn_01hjryxysgey07h5jz5wagqj0m
          userId: us_01ke1h5471vxsbscp8jd3bramn
          newRole: INVALID_ROLE
  expect:
    status: 200
    contains:
    - errors
    - OrgRole
    - does not contain the value
```
