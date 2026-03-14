---
name: リポジトリメンバー権限が正しく動作する
description: 'リポジトリメンバーの招待・ロール変更による権限の付与・剥奪が正しく動作することを検証する。

  AdminPolicyを持たないtest2ユーザーを使用して、純粋なリポジトリメンバー権限をテストする。

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

# リポジトリメンバー権限が正しく動作する

リポジトリメンバーの招待・ロール変更による権限の付与・剥奪が正しく動作することを検証する。
AdminPolicyを持たないtest2ユーザーを使用して、純粋なリポジトリメンバー権限をテストする。


## テスト用組織を作成する（adminユーザー）

```yaml scenario
steps:
- id: create_org
  name: テスト用組織を作成する（adminユーザー）
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateOrg($input: CreateOrganizationInput!) {\n  createOrganization(input: $input) {\n    id\n    username\n  }\n}\n"
      variables:
        input:
          name: Repo Member Test Org {{vars.timestamp}}
          username: repo-member-test-{{vars.timestamp}}
          description: Organization for repo member permission test
  expect:
    status: 200
    contains:
    - '"createOrganization"'
    - repo-member-test-{{vars.timestamp}}
```

## プライベートリポジトリを作成する（adminユーザー）

```yaml scenario
steps:
- id: create_private_repo
  name: プライベートリポジトリを作成する（adminユーザー）
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation CreateRepo($input: CreateRepoInput!) {\n  createRepo(input: $input) {\n    id\n    username\n    isPublic\n  }\n}\n"
      variables:
        input:
          orgUsername: repo-member-test-{{vars.timestamp}}
          repoName: Permission Test Repo {{vars.timestamp}}
          repoUsername: perm-test-repo-{{vars.timestamp}}
          userId: us_01hs2yepy5hw4rz8pdq2wywnwt
          isPublic: false
          description: Private repo for permission test
  expect:
    status: 200
    contains:
    - '"createRepo"'
    - perm-test-repo-{{vars.timestamp}}
    - '"isPublic":false'
```

## 非メンバー（test2）はリポジトリを更新できない

```yaml scenario
steps:
- id: non_member_cannot_update
  name: 非メンバー（test2）はリポジトリを更新できない
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01ke1h5471vxsbscp8jd3bramn
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n  }\n}\n"
      variables:
        input:
          orgUsername: repo-member-test-{{vars.timestamp}}
          repoUsername: perm-test-repo-{{vars.timestamp}}
          name: Should Fail
          description: This update should fail - user is not a member
  expect:
    status: 200
    contains:
    - errors
    - PermissionDenied
```

## test2をwriterロールで招待する

```yaml scenario
steps:
- id: invite_as_writer
  name: test2をwriterロールで招待する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation InviteMember($input: InviteRepoMemberInput!) {\n  inviteRepoMember(input: $input)\n}\n"
      variables:
        input:
          orgUsername: repo-member-test-{{vars.timestamp}}
          repoUsername: perm-test-repo-{{vars.timestamp}}
          repoId: {{steps.create_private_repo.outputs.createRepo.id}}
          usernameOrEmail: test2
          role: writer
  expect:
    status: 200
    contains:
    - '"inviteRepoMember":true'
```

## writerロールはリポジトリを更新できる

```yaml scenario
steps:
- id: writer_can_update
  name: writerロールはリポジトリを更新できる
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01ke1h5471vxsbscp8jd3bramn
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n    description\n  }\n}\n"
      variables:
        input:
          orgUsername: repo-member-test-{{vars.timestamp}}
          repoUsername: perm-test-repo-{{vars.timestamp}}
          name: Updated by Writer
          description: Updated by test2 as writer
  expect:
    status: 200
    contains:
    - '"updateRepo"'
    - Updated by Writer
```

## test2をreaderロールに降格する

```yaml scenario
steps:
- id: downgrade_to_reader
  name: test2をreaderロールに降格する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation ChangeRole($input: ChangeRepoMemberRoleInput!) {\n  changeRepoMemberRole(input: $input)\n}\n"
      variables:
        input:
          repoId: {{steps.create_private_repo.outputs.createRepo.id}}
          userId: us_01ke1h5471vxsbscp8jd3bramn
          newRole: reader
  expect:
    status: 200
    contains:
    - '"changeRepoMemberRole":true'
```

## readerロールはリポジトリを更新できない

```yaml scenario
steps:
- id: reader_cannot_update
  name: readerロールはリポジトリを更新できない
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01ke1h5471vxsbscp8jd3bramn
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n  }\n}\n"
      variables:
        input:
          orgUsername: repo-member-test-{{vars.timestamp}}
          repoUsername: perm-test-repo-{{vars.timestamp}}
          name: Should Fail
          description: This update should fail - reader has no write access
  expect:
    status: 200
    contains:
    - errors
    - PermissionDenied
```

## test2をownerロールに昇格する

```yaml scenario
steps:
- id: upgrade_to_owner
  name: test2をownerロールに昇格する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation ChangeRole($input: ChangeRepoMemberRoleInput!) {\n  changeRepoMemberRole(input: $input)\n}\n"
      variables:
        input:
          repoId: {{steps.create_private_repo.outputs.createRepo.id}}
          userId: us_01ke1h5471vxsbscp8jd3bramn
          newRole: owner
  expect:
    status: 200
    contains:
    - '"changeRepoMemberRole":true'
```

## ownerロールはリポジトリを更新できる

```yaml scenario
steps:
- id: owner_can_update
  name: ownerロールはリポジトリを更新できる
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01ke1h5471vxsbscp8jd3bramn
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n    description\n  }\n}\n"
      variables:
        input:
          orgUsername: repo-member-test-{{vars.timestamp}}
          repoUsername: perm-test-repo-{{vars.timestamp}}
          name: Updated by Owner
          description: Updated by test2 as owner
  expect:
    status: 200
    contains:
    - '"updateRepo"'
    - Updated by Owner
```

## test2をメンバーから削除する

```yaml scenario
steps:
- id: remove_member
  name: test2をメンバーから削除する
  request:
    method: POST
    url: /v1/graphql
    body:
      query: "mutation RemoveMember($input: RemoveRepoMemberInput!) {\n  removeRepoMember(input: $input)\n}\n"
      variables:
        input:
          repoId: {{steps.create_private_repo.outputs.createRepo.id}}
          userId: us_01ke1h5471vxsbscp8jd3bramn
  expect:
    status: 200
    contains:
    - '"removeRepoMember":true'
```

## 削除されたメンバーはリポジトリを更新できない

```yaml scenario
steps:
- id: removed_member_cannot_update
  name: 削除されたメンバーはリポジトリを更新できない
  request:
    method: POST
    url: /v1/graphql
    headers:
      x-user-id: us_01ke1h5471vxsbscp8jd3bramn
    body:
      query: "mutation UpdateRepo($input: UpdateRepoInput!) {\n  updateRepo(input: $input) {\n    username\n    name\n  }\n}\n"
      variables:
        input:
          orgUsername: repo-member-test-{{vars.timestamp}}
          repoUsername: perm-test-repo-{{vars.timestamp}}
          name: Should Fail
          description: This update should fail - user is no longer a member
  expect:
    status: 200
    contains:
    - errors
    - PermissionDenied
```
