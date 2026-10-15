import { externalSyncRequest, type ExternalSyncBinding, type ExternalSyncTarget } from './externalSyncApi'

export interface GitHubDeliveryStatus {
  id: string
  guid: string
  deliveredAt: string | null
  status: string | null
  statusCode: number | null
  redelivery: boolean
}

export interface GitHubManagedWebhookStatus {
  endpointId: string
  webhookUrl: string
  hookId: string | null
  active: boolean | null
  lastResponseCode: number | null
  lastResponseStatus: string | null
  deliveries: GitHubDeliveryStatus[]
}

function requireBinding(target: ExternalSyncTarget, binding: ExternalSyncBinding) {
  if (!target.operatorId || target.repositoryId !== binding.repositoryId || binding.provider !== 'GITHUB') {
    throw new Error('Invalid GitHub binding')
  }
}

export async function getGitHubManagedWebhook(target: ExternalSyncTarget, binding: ExternalSyncBinding) {
  requireBinding(target, binding)
  const data = await externalSyncRequest<{ githubBindingWebhookStatus: GitHubManagedWebhookStatus | null }>(target,
    `query GitHubBindingWebhookStatus($bindingId: String!) {
      githubBindingWebhookStatus(bindingId: $bindingId) {
        endpointId webhookUrl hookId active lastResponseCode lastResponseStatus
        deliveries { id guid deliveredAt status statusCode redelivery }
      }
    }`, { bindingId: binding.id })
  return data.githubBindingWebhookStatus
}

export async function ensureGitHubManagedWebhook(target: ExternalSyncTarget, binding: ExternalSyncBinding) {
  requireBinding(target, binding)
  await externalSyncRequest(target,
    `mutation ProvisionGitHubBindingWebhook($bindingId: String!) {
      provisionGithubBindingWebhook(bindingId: $bindingId) { hookId endpointId }
    }`, { bindingId: binding.id })
}

export async function removeGitHubManagedWebhook(target: ExternalSyncTarget, binding: ExternalSyncBinding) {
  requireBinding(target, binding)
  await externalSyncRequest(target,
    `mutation RemoveGitHubBindingWebhook($bindingId: String!) {
      removeGithubBindingWebhook(bindingId: $bindingId)
    }`, { bindingId: binding.id })
}

export async function redeliverGitHubManagedWebhook(target: ExternalSyncTarget, binding: ExternalSyncBinding, deliveryId: string) {
  requireBinding(target, binding)
  if (!/^\d+$/.test(deliveryId)) throw new Error('Invalid delivery ID')
  await externalSyncRequest(target,
    `mutation RedeliverGitHubBindingWebhook($bindingId: String!, $deliveryId: String!) {
      redeliverGithubBindingWebhook(bindingId: $bindingId, deliveryId: $deliveryId)
    }`, { bindingId: binding.id, deliveryId })
}
