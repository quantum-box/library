---
name: Public docs routes enforce repo visibility
description: |
  `/docs/{org}/{repo}` routes expose public repos anonymously, allow private
  repos only for authorized users, and return composed Markdown with YAML
  frontmatter for indexing.
config:
  headers:
    Authorization: Bearer dummy-token
    Content-Type: application/json
    x-platform-id: tn_01j702qf86pc2j35s0kv0gv3gy
    x-user-id: us_01hs2yepy5hw4rz8pdq2wywnwt
  timeout: 30
  continue_on_failure: false
---

# Public docs routes enforce repo visibility

## Create organization

```yaml scenario
steps:
- id: create_org
  name: Create organization
  request:
    method: POST
    url: /v1beta/orgs
    body:
      name: Docs Route Org {{vars.timestamp}}
      username: docs-route-org-{{vars.timestamp}}
      description: docs route org
  expect:
    status: 200
    contains:
    - docs-route-org-{{vars.timestamp}}
  save:
    org_username: username
```

## Create public repo

```yaml scenario
steps:
- id: create_public_repo
  name: Create public repo
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}
    body:
      name: Public Docs Repo {{vars.timestamp}}
      username: public-docs-{{vars.timestamp}}
      description: public docs repo
      is_public: true
  expect:
    status: 200
    contains:
    - public-docs-{{vars.timestamp}}
  save:
    public_repo_username: username
```

## Create private repo

```yaml scenario
steps:
- id: create_private_repo
  name: Create private repo
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}
    body:
      name: Private Docs Repo {{vars.timestamp}}
      username: private-docs-{{vars.timestamp}}
      description: private docs repo
      is_public: false
  expect:
    status: 200
    contains:
    - private-docs-{{vars.timestamp}}
  save:
    private_repo_username: username
```

## Add public markdown content property

```yaml scenario
steps:
- id: add_public_content_property
  name: Add public markdown content property
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.public_repo_username}}/properties
    body:
      name: content
      property_type: markdown
  expect:
    status: 200
    contains:
    - content
  save:
    public_content_property_id: id
```

## Add public slug property

```yaml scenario
steps:
- id: add_public_slug_property
  name: Add public slug property
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.public_repo_username}}/properties
    body:
      name: slug
      property_type: string
  expect:
    status: 200
    contains:
    - slug
  save:
    public_slug_property_id: id
```

## Add public document

```yaml scenario
steps:
- id: add_public_doc
  name: Add public document
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.public_repo_username}}/data
    body:
      name: Public GA Doc {{vars.timestamp}}
      property_data:
      - property_id: {{vars.public_content_property_id}}
        value:
          markdown: "# Public GA Doc\n\nsearch-index-body-{{vars.timestamp}}"
      - property_id: {{vars.public_slug_property_id}}
        value:
          string: public-ga-doc-{{vars.timestamp}}
  expect:
    status: 200
    contains:
    - Public GA Doc
  save:
    public_data_id: id
```

## Add private markdown content property

```yaml scenario
steps:
- id: add_private_content_property
  name: Add private markdown content property
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.private_repo_username}}/properties
    body:
      name: content
      property_type: markdown
  expect:
    status: 200
    contains:
    - content
  save:
    private_content_property_id: id
```

## Add private document

```yaml scenario
steps:
- id: add_private_doc
  name: Add private document
  request:
    method: POST
    url: /v1beta/repos/{{vars.org_username}}/{{vars.private_repo_username}}/data
    body:
      name: Private GA Doc {{vars.timestamp}}
      property_data:
      - property_id: {{vars.private_content_property_id}}
        value:
          markdown: "# Private GA Doc\n\nprivate-body-{{vars.timestamp}}"
  expect:
    status: 200
    contains:
    - Private GA Doc
  save:
    private_data_id: id
```

## Anonymous list public docs

```yaml scenario
steps:
- id: anon_list_public_docs
  name: Anonymous list public docs
  request:
    method: GET
    url: /docs/{{vars.org_username}}/{{vars.public_repo_username}}
    headers:
      Authorization: ''
  expect:
    status: 200
    contains:
    - Public GA Doc {{vars.timestamp}}
    - /docs/{{vars.org_username}}/{{vars.public_repo_username}}/{{vars.public_data_id}}
```

## Anonymous view public doc HTML

```yaml scenario
steps:
- id: anon_view_public_doc
  name: Anonymous view public doc HTML
  request:
    method: GET
    url: /docs/{{vars.org_username}}/{{vars.public_repo_username}}/{{vars.public_data_id}}
    headers:
      Authorization: ''
  expect:
    status: 200
    contains:
    - '<h1>Public GA Doc</h1>'
    - search-index-body-{{vars.timestamp}}
```

## Anonymous view public doc Markdown

```yaml scenario
steps:
- id: anon_view_public_doc_md
  name: Anonymous view public doc Markdown
  request:
    method: GET
    url: /docs/{{vars.org_username}}/{{vars.public_repo_username}}/{{vars.public_data_id}}/md
    headers:
      Authorization: ''
  expect:
    status: 200
    contains:
    - '---'
    - 'title: Public GA Doc {{vars.timestamp}}'
    - 'slug: public-ga-doc-{{vars.timestamp}}'
    - '# Public GA Doc'
```

## Anonymous list private docs is forbidden

```yaml scenario
steps:
- id: anon_list_private_docs
  name: Anonymous list private docs is forbidden
  request:
    method: GET
    url: /docs/{{vars.org_username}}/{{vars.private_repo_username}}
    headers:
      Authorization: ''
  expect:
    status: 403
    contains:
    - Access denied
```

## Authorized list private docs succeeds

```yaml scenario
steps:
- id: auth_list_private_docs
  name: Authorized list private docs succeeds
  request:
    method: GET
    url: /docs/{{vars.org_username}}/{{vars.private_repo_username}}
  expect:
    status: 200
    contains:
    - Private GA Doc {{vars.timestamp}}
```

## Authorized private Markdown succeeds

```yaml scenario
steps:
- id: auth_view_private_doc_md
  name: Authorized private Markdown succeeds
  request:
    method: GET
    url: /docs/{{vars.org_username}}/{{vars.private_repo_username}}/{{vars.private_data_id}}/md
  expect:
    status: 200
    contains:
    - 'title: Private GA Doc {{vars.timestamp}}'
    - '# Private GA Doc'
```

## Missing document returns 404

```yaml scenario
steps:
- id: missing_public_doc
  name: Missing document returns 404
  request:
    method: GET
    url: /docs/{{vars.org_username}}/{{vars.public_repo_username}}/dt_missing_{{vars.timestamp}}
    headers:
      Authorization: ''
  expect:
    status: 404
```
