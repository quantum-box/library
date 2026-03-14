'use server'

import { executeGraphQL, graphql } from '@/lib/graphql'
import { EndpointDetailUI } from './components/endpoint-detail-ui'

const WebhookEndpointQuery = graphql(`
  query WebhookEndpoint($tenantId: String!, $endpointId: String!) {
    webhookEndpoint(tenantId: $tenantId, endpointId: $endpointId) {
      id
      tenantId
      repositoryId
      name
      provider
      config
      events
      mapping
      status
      webhookUrl
      createdAt
      updatedAt
    }
  }
`)

const WebhookEventsQuery = graphql(`
  query WebhookEvents($tenantId: String!, $endpointId: String!, $limit: Int, $offset: Int) {
    webhookEvents(tenantId: $tenantId, endpointId: $endpointId, limit: $limit, offset: $offset) {
      id
      endpointId
      provider
      eventType
      payload
      signatureValid
      status
      errorMessage
      retryCount
      stats {
        created
        updated
        deleted
        skipped
        total
      }
      receivedAt
      processedAt
    }
  }
`)

interface PageProps {
	params: Promise<{ org: string; endpointId: string }>
	searchParams: Promise<{ page?: string }>
}

export default async function EndpointDetailPage({
	params,
	searchParams,
}: PageProps) {
	const { org, endpointId } = await params
	const { page } = await searchParams
	const currentPage = Number.parseInt(page || '1', 10)
	const limit = 20
	const offset = (currentPage - 1) * limit

	const [endpointResult, eventsResult] = await Promise.all([
		executeGraphQL(WebhookEndpointQuery, {
			tenantId: org,
			endpointId,
		}),
		executeGraphQL(WebhookEventsQuery, {
			tenantId: org,
			endpointId,
			limit,
			offset,
		}),
	])

	const endpoint = endpointResult?.webhookEndpoint
	const events = eventsResult?.webhookEvents ?? []

	if (!endpoint) {
		return (
			<div className='container mx-auto py-6'>
				<h1 className='text-2xl font-bold'>Endpoint not found</h1>
			</div>
		)
	}

	return (
		<EndpointDetailUI
			tenantId={org}
			endpoint={endpoint}
			events={events}
			currentPage={currentPage}
			hasMore={events.length === limit}
		/>
	)
}
