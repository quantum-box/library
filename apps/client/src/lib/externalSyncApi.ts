import {
  configuredLibraryApiBaseUrl,
  libraryGraphqlHeaders,
  type LibraryGraphqlError,
} from './libraryGraphql'

export interface ExternalSyncTarget {
  repositoryId: string
  operatorId?: string
}

export interface ExternalSyncBinding {
  id: string
  repositoryId: string
  provider: string
  connectionId: string
  externalScope: string
  objectType: string
  inboundPolicy: string
  outboundPolicy: string
  deletePolicy: string
  status: 'ACTIVE' | 'PAUSED' | 'REAUTHORIZATION_REQUIRED'
  updatedAt: string
}

export interface InboundChangeSet {
  id: string
  bindingId: string
  dataId?: string | null
  externalObjectId: string
  externalRevision: string
  baseExternalRevision?: string | null
  changeType: string
  status: string
  decisionNote?: string | null
  createdAt: string
}

export interface OutboundDelivery {
  id: string
  bindingId: string
  dataId: string
  externalObjectId: string
  libraryRevision: string
  status: string
  attemptCount: number
  nextAttemptAt: string
  lastErrorCategory?: string | null
  deliveryUrl?: string | null
  updatedAt: string
}

export interface ExternalSyncOverview {
  bindings: ExternalSyncBinding[]
  changes: InboundChangeSet[]
  deliveries: OutboundDelivery[]
}

export class ExternalSyncApiError extends Error {
  readonly status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'ExternalSyncApiError'
    this.status = status
  }
}

async function request<T>(
  target: ExternalSyncTarget,
  query: string,
  variables: Record<string, unknown>,
): Promise<T> {
  let response: Response
  try {
    response = await fetch(`${configuredLibraryApiBaseUrl()}/v1/graphql`, {
      method: 'POST',
      headers: await libraryGraphqlHeaders(target.operatorId),
      body: JSON.stringify({ query, variables }),
    })
  } catch (error) {
    const detail = error instanceof Error ? `: ${error.message}` : ''
    throw new ExternalSyncApiError(`External sync is unavailable${detail}`, 0)
  }
  if (!response.ok) {
    throw new ExternalSyncApiError(
      `External sync request failed: ${response.status}`,
      response.status,
    )
  }
  const payload = await response.json() as { data?: T; errors?: LibraryGraphqlError[] }
  if (payload.errors?.length || payload.data == null) {
    throw new ExternalSyncApiError(
      payload.errors?.[0]?.message ?? 'External sync returned an invalid response.',
      payload.errors?.[0]?.extensions?.status ?? 400,
    )
  }
  return payload.data
}

const BINDING_FIELDS = `
  id repositoryId provider connectionId externalScope objectType
  inboundPolicy outboundPolicy deletePolicy status updatedAt
`

const CHANGE_FIELDS = `
  id bindingId dataId externalObjectId externalRevision baseExternalRevision
  changeType status decisionNote createdAt
`

const DELIVERY_FIELDS = `
  id bindingId dataId externalObjectId libraryRevision status attemptCount
  nextAttemptAt lastErrorCategory deliveryUrl updatedAt
`

export async function fetchExternalSyncOverview(
  target: ExternalSyncTarget,
): Promise<ExternalSyncOverview> {
  const bindingData = await request<{ externalSyncBindings: ExternalSyncBinding[] }>(
    target,
    `query ExternalSyncBindings($repositoryId: String!) {
      externalSyncBindings(repositoryId: $repositoryId) { ${BINDING_FIELDS} }
    }`,
    { repositoryId: target.repositoryId },
  )
  const results = await Promise.all(bindingData.externalSyncBindings.map(async (binding) => {
    const data = await request<{
      inboundChangeSets: InboundChangeSet[]
      outboundDeliveries: OutboundDelivery[]
    }>(target, `query ExternalSyncActivity($bindingId: String!) {
      inboundChangeSets(bindingId: $bindingId, limit: 50) { ${CHANGE_FIELDS} }
      outboundDeliveries(bindingId: $bindingId, limit: 50) { ${DELIVERY_FIELDS} }
    }`, { bindingId: binding.id })
    return data
  }))
  return {
    bindings: bindingData.externalSyncBindings,
    changes: results.flatMap((result) => result.inboundChangeSets),
    deliveries: results.flatMap((result) => result.outboundDeliveries),
  }
}

export async function setExternalSyncBindingStatus(
  target: ExternalSyncTarget,
  bindingId: string,
  status: 'ACTIVE' | 'PAUSED',
): Promise<ExternalSyncBinding> {
  const data = await request<{ updateExternalSyncBindingStatus: ExternalSyncBinding }>(
    target,
    `mutation UpdateExternalSyncBindingStatus($bindingId: String!, $status: GqlExternalSyncBindingStatus!) {
      updateExternalSyncBindingStatus(bindingId: $bindingId, status: $status) { ${BINDING_FIELDS} }
    }`,
    { bindingId, status },
  )
  return data.updateExternalSyncBindingStatus
}

export async function decideInboundChange(
  target: ExternalSyncTarget,
  changeSetId: string,
  accept: boolean,
): Promise<InboundChangeSet> {
  const data = await request<{ decideInboundChangeSet: InboundChangeSet }>(
    target,
    `mutation DecideInboundChangeSet($changeSetId: String!, $accept: Boolean!) {
      decideInboundChangeSet(changeSetId: $changeSetId, accept: $accept) { ${CHANGE_FIELDS} }
    }`,
    { changeSetId, accept },
  )
  return data.decideInboundChangeSet
}

export async function retryOutboundDelivery(
  target: ExternalSyncTarget,
  deliveryId: string,
): Promise<OutboundDelivery> {
  const data = await request<{ retryOutboundDelivery: OutboundDelivery }>(
    target,
    `mutation RetryOutboundDelivery($deliveryId: String!) {
      retryOutboundDelivery(deliveryId: $deliveryId) { ${DELIVERY_FIELDS} }
    }`,
    { deliveryId },
  )
  return data.retryOutboundDelivery
}
