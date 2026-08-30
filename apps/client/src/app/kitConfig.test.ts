import { describe, expect, it } from 'vitest'
import {
  appKitConfig,
  buildRoomId,
  buildUserStorageScope,
  buildWorkspaceScope,
  buildSyncWebsocketPath,
  namespacedKey,
  resolveAppServerBackend,
  resolveChatStreamMode,
  resolveChatStreamTransport,
  resolveDeploymentMode,
  resolveFrontendWorkerRuntime,
  resolveBrowserSyncWebsocketUrl,
  resolveSyncBackend,
  resolveTenantWorkspaceOptions,
  resolveTenantWorkspaceSelection,
} from './kitConfig'

describe('appKitConfig', () => {
  it('defines the project-specific extension points for the client app kit', () => {
    expect(appKitConfig.app.id).toBe('library-client')
    expect(appKitConfig.app.storageNamespace).toBe('library-client')
    expect(appKitConfig.tenant.id).toBe('library')
    expect(appKitConfig.tenancy.availableWorkspaces).toEqual([
      {
        tenantId: 'library',
        tenantName: 'Library',
        workspaceId: 'library-default',
        workspaceName: 'Library',
        workspaceInitial: 'L',
      },
    ])
    expect(appKitConfig.tenancy.selectionStorageKey).toBe('library-client-tenant-workspace')
    expect(appKitConfig.workspace.name).toBe('Library')
    expect(appKitConfig.workspace.initial).toHaveLength(1)
    expect(appKitConfig.workspace.id).toBe('library-default')
    expect(appKitConfig.workspace.scope).toBe('tenant:library:workspace:library-default')
    expect(appKitConfig.workspace.primaryNav.length).toBeGreaterThan(0)
    expect(appKitConfig.workspace.projects.length).toBeGreaterThan(0)
    expect(appKitConfig.workspace.users.length).toBeGreaterThan(0)
  })

  it('keeps runtime storage and sync keys explicit', () => {
    expect(appKitConfig.storage.themeKey).toBe('library-client-theme')
    expect(appKitConfig.sync.backend).toBe('rust-server')
    expect(appKitConfig.sync.tenantId).toBe('library')
    expect(appKitConfig.sync.workspaceId).toBe('library-default')
    expect(appKitConfig.sync.workspaceScope).toBe('tenant:library:workspace:library-default')
    expect(appKitConfig.sync.recordsRoomId).toBe('tenant:library:workspace:library-default:records')
    expect(appKitConfig.sync.persistenceKey).toBe('tenant:library:workspace:library-default:records')
    expect(appKitConfig.sync.yjsArrayName).toBe('records')
    expect(appKitConfig.sync.databasesArrayName).toBe('databases')
    expect(appKitConfig.sync.workflowCanvasesMapName).toBe('workflowCanvases')
    expect(appKitConfig.sync.websocketPath).toBe(
      '/ws?room=tenant:library:workspace:library-default:records'
    )
    expect(appKitConfig.sync.websocketUrl).toBeUndefined()
    expect(appKitConfig.workflows.pgliteDataDir).toBe(
      'idb://library-workflows-tenant-library-workspace-library-default'
    )
    expect(appKitConfig.docs.pgliteDataDir).toBe(
      'idb://library-docs-tenant-library-workspace-library-default'
    )
    expect(appKitConfig.engine.pgliteDataDir).toBe(
      'idb://library-engine-tenant-library-workspace-library-default'
    )
    expect(appKitConfig.engine.pushPath).toBe('/api/engine/push')
    expect(appKitConfig.engine.pullPath).toBe('/api/engine/pull')
    expect(appKitConfig.docs.yjsArrayName).toBe('blocks')
    expect(appKitConfig.attachments.yjsArrayName).toBe('attachments')
    expect(appKitConfig.attachments.webStorageProvider).toBe('web-object-storage')
    expect(appKitConfig.sync.roomParam).toBe('room')
    expect(appKitConfig.server.recordsPath).toBe('/api/records')
    expect(appKitConfig.chat.stream.mode).toBe('mock')
    expect(appKitConfig.chat.stream.transport).toBe('sse')
    expect(appKitConfig.chat.stream.endpoint).toBe('/api/agent/chat/stream')
    expect(appKitConfig.chat.stream.toolResultPath).toBe('/api/agent/tool-results')
  })

  it('isolates local-first storage by user without changing the shared workspace scope', () => {
    const workspaceScope = buildWorkspaceScope('tenant-a', 'workspace-a')

    expect(buildUserStorageScope(workspaceScope, undefined)).toBe(workspaceScope)
    expect(buildUserStorageScope(workspaceScope, 'user:01/example')).toBe(
      'tenant:tenant-a:workspace:workspace-a:user:user-01-example',
    )
  })

  it('keeps the Cloudflare worker as a required frontend-side component', () => {
    expect(appKitConfig.frontendWorker.enabled).toBe(true)
    expect(appKitConfig.frontendWorker.role).toBe('frontend-edge-companion')
    expect(appKitConfig.frontendWorker.websocketPath).toBe(appKitConfig.sync.websocketPath)
    expect(appKitConfig.frontendWorker.healthPath).toBe('/api/health')
  })

  it('resolves deployment defaults for local, cloud, and on-premise installs', () => {
    expect(resolveDeploymentMode(undefined)).toBe('local')
    expect(resolveDeploymentMode('cloud')).toBe('cloud')
    expect(resolveDeploymentMode('onprem')).toBe('onprem')
    expect(resolveDeploymentMode('invalid')).toBe('local')

    expect(resolveSyncBackend('local', undefined)).toBe('rust-server')
    expect(resolveSyncBackend('cloud', undefined)).toBe('cloudflare-durable-object')
    expect(resolveSyncBackend('onprem', undefined)).toBe('rust-server')
    expect(resolveSyncBackend('onprem', 'cloudflare-durable-object')).toBe(
      'cloudflare-durable-object'
    )

    expect(resolveFrontendWorkerRuntime('cloud', undefined)).toBe('cloudflare-workers')
    expect(resolveFrontendWorkerRuntime('onprem', undefined)).toBe('workerd')
    expect(resolveFrontendWorkerRuntime('onprem', 'cloudflare-workers')).toBe(
      'cloudflare-workers'
    )

    expect(resolveAppServerBackend(undefined)).toBe('rust-server')
    expect(resolveAppServerBackend('external-api')).toBe('external-api')

    expect(resolveChatStreamMode('local', undefined)).toBe('mock')
    expect(resolveChatStreamMode('cloud', undefined)).toBe('mock')
    expect(resolveChatStreamMode('onprem', undefined)).toBe('mock')
    expect(resolveChatStreamMode('cloud', 'backend')).toBe('backend')
    expect(resolveChatStreamMode('onprem', 'backend')).toBe('backend')
    expect(resolveChatStreamMode('onprem', 'mock')).toBe('mock')
    expect(resolveChatStreamTransport(undefined)).toBe('sse')
    expect(resolveChatStreamTransport('websocket')).toBe('websocket')
  })

  it('builds storage keys from the app namespace', () => {
    expect(namespacedKey('another-app', 'theme')).toBe('another-app-theme')
    expect(appKitConfig.storage.themeKey).toBe(
      namespacedKey(appKitConfig.app.storageNamespace, 'theme')
    )
  })

  it('configures record defaults without hardcoding them in UI components', () => {
    expect(appKitConfig.records.identifierPrefix).toMatch(/^[A-Z]+$/)
    expect(appKitConfig.records.defaultProject).toBeTruthy()
  })

  it('defines a configurable business workflow for records', () => {
    const workflow = appKitConfig.workflows.definitions.find(
      (definition) => definition.id === appKitConfig.workflows.defaultWorkflowId
    )

    expect(workflow?.name).toBe('Default Data Workflow')
    expect(workflow?.stages.map((stage) => stage.id)).toEqual([
      'intake',
      'ready',
      'execution',
      'validation',
      'completed',
      'cancelled',
    ])
    expect(workflow?.transitions.some((transition) => transition.kind === 'exception')).toBe(true)
  })
})

describe('tenant workspace selection', () => {
  const fallback = {
    tenantId: 'photon',
    tenantName: 'Photon',
    workspaceId: 'photon-default',
    workspaceName: 'Photon',
    workspaceInitial: 'P',
  }

  it('parses available tenant workspaces from JSON config', () => {
    expect(resolveTenantWorkspaceOptions(JSON.stringify([
      {
        tenantId: 'acme',
        tenantName: 'Acme',
        workspaceId: 'roadmap',
        workspaceName: 'Roadmap',
        workspaceInitial: 'R',
      },
      {
        tenantId: 'globex',
        tenantName: 'Globex',
        workspaceId: 'ops',
        workspaceName: 'Operations',
      },
    ]), fallback)).toEqual([
      {
        tenantId: 'acme',
        tenantName: 'Acme',
        workspaceId: 'roadmap',
        workspaceName: 'Roadmap',
        workspaceInitial: 'R',
      },
      {
        tenantId: 'globex',
        tenantName: 'Globex',
        workspaceId: 'ops',
        workspaceName: 'Operations',
        workspaceInitial: 'O',
      },
    ])
  })

  it('selects the requested tenant/workspace and falls back safely', () => {
    const options = resolveTenantWorkspaceOptions(JSON.stringify([
      { tenantId: 'acme', tenantName: 'Acme', workspaceId: 'roadmap', workspaceName: 'Roadmap' },
      { tenantId: 'globex', tenantName: 'Globex', workspaceId: 'ops', workspaceName: 'Operations' },
    ]), fallback)

    expect(resolveTenantWorkspaceSelection(options, 'globex', 'ops').workspaceName).toBe('Operations')
    expect(resolveTenantWorkspaceSelection(options, 'missing', 'ops')).toBe(options[0])
  })
})

describe('buildRoomId', () => {
  it('encodes workspace scope with tenant and workspace boundaries', () => {
    expect(buildWorkspaceScope('photon', 'photon-default')).toBe(
      'tenant:photon:workspace:photon-default'
    )
  })

  it('encodes the record room as tenant:<id>:workspace:<id>:records', () => {
    expect(buildRoomId('tenant:photon:workspace:photon-default', 'records')).toBe(
      'tenant:photon:workspace:photon-default:records'
    )
  })

  it('supports composite surfaces such as docs and chat threads', () => {
    const scope = buildWorkspaceScope('acme-corp', 'acme')
    expect(buildRoomId(scope, 'doc:42')).toBe('tenant:acme-corp:workspace:acme:doc:42')
    expect(buildRoomId(scope, 'chat:general')).toBe(
      'tenant:acme-corp:workspace:acme:chat:general'
    )
  })

  it('builds room-scoped websocket paths', () => {
    const path = buildSyncWebsocketPath('tenant:acme-corp:workspace:acme:doc:42')
    expect(path).toBe('/ws?room=tenant:acme-corp:workspace:acme:doc:42')
    expect(resolveBrowserSyncWebsocketUrl(undefined, path, {
      protocol: 'http:',
      host: '127.0.0.1:5173',
    })).toBe('ws://127.0.0.1:5173/ws?room=tenant:acme-corp:workspace:acme:doc:42')
    expect(resolveBrowserSyncWebsocketUrl(undefined, path, {
      protocol: 'https:',
      host: 'library.example.test',
    })).toBe('wss://library.example.test/ws?room=tenant:acme-corp:workspace:acme:doc:42')
  })

  it('keeps an explicit sync WebSocket URL instead of replacing its origin', () => {
    expect(resolveBrowserSyncWebsocketUrl(
      'wss://sync.example.test/ws?room=tenant:acme:workspace:roadmap:records',
      '/ws?room=ignored',
      { protocol: 'https:', host: 'library.example.test' },
    )).toBe('wss://sync.example.test/ws?room=tenant:acme:workspace:roadmap:records')
  })
})
