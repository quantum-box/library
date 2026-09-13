import { externalSyncRequest, type ExternalSyncBinding, type ExternalSyncTarget } from './externalSyncApi'
import { isGitHubScopeValid } from './githubSyncSetup'

export interface GitHubWebhookEndpoint {
  id: string
  config: string
  status: string
  webhookUrl: string
}

const fields = 'id config status webhookUrl'

export function githubWebhookScope(binding: ExternalSyncBinding) {
  const scope = JSON.parse(binding.externalScope)
  const result = { repository: scope.repository, branch: scope.ref, pathPattern: scope.path_pattern ?? '' }
  if (binding.provider !== 'GITHUB' || typeof result.repository !== 'string' || typeof result.branch !== 'string'
    || typeof result.pathPattern !== 'string' || !isGitHubScopeValid(result)) throw new Error('Invalid GitHub binding')
  return result
}

export async function findGitHubWebhook(target: ExternalSyncTarget, binding: ExternalSyncBinding) {
  if (!target.operatorId || binding.repositoryId !== target.repositoryId) throw new Error('Invalid webhook organization or repository')
  const scope = githubWebhookScope(binding)
  const data = await externalSyncRequest<{ webhookEndpoints: GitHubWebhookEndpoint[] }>(target,
    `query GitHubWebhookEndpoints($tenantId: String!, $repositoryId: String!) {
      webhookEndpoints(tenantId: $tenantId, repositoryId: $repositoryId, provider: GITHUB) { ${fields} }
    }`, { tenantId: target.operatorId, repositoryId: target.repositoryId })
  return data.webhookEndpoints.find((endpoint) => {
    try {
      const config = JSON.parse(endpoint.config)
      return config.repository === scope.repository && config.branch === scope.branch
        && (config.path_pattern ?? '') === scope.pathPattern
    } catch { return false }
  }) ?? null
}

export async function prepareGitHubWebhook(target: ExternalSyncTarget, binding: ExternalSyncBinding):
  Promise<{ endpoint: GitHubWebhookEndpoint; secret?: string }> {
  // Reconcile first after a lost response; do not duplicate a live receiver.
  const existing = await findGitHubWebhook(target, binding)
  if (existing) return { endpoint: existing }
  const scope = githubWebhookScope(binding)
  const data = await externalSyncRequest<{ createWebhookEndpoint: { endpoint: GitHubWebhookEndpoint; secret: string } }>(target,
    `mutation PrepareGitHubWebhook($input: CreateWebhookEndpointInput!) {
      createWebhookEndpoint(input: $input) { endpoint { ${fields} } secret }
    }`, { input: {
      name: `${scope.repository} (${scope.branch})`, provider: 'GITHUB', repositoryId: target.repositoryId,
      config: JSON.stringify({ provider: 'github', repository: scope.repository, branch: scope.branch, path_pattern: scope.pathPattern || null }),
      events: ['push', 'pull_request'],
    } })
  return data.createWebhookEndpoint
}
