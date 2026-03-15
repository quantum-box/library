'use server'

export const runtime = 'edge'

import { executeGraphQL, graphql } from '@/lib/graphql'
import { WebhooksPageUI } from './components/webhooks-page-ui'

const WebhookEndpointsQuery = graphql(`
  query WebhookEndpoints($tenantId: String!) {
    webhookEndpoints(tenantId: $tenantId) {
      id
      tenantId
      repositoryId
      name
      provider
      events
      status
      webhookUrl
      createdAt
      updatedAt
    }
  }
`)

interface PageProps {
	params: Promise<{ org: string }>
}

export default async function WebhooksPage({ params }: PageProps) {
	const { org } = await params

	const result = await executeGraphQL(WebhookEndpointsQuery, {
		tenantId: org,
	})

	const endpoints = result?.webhookEndpoints ?? []

	return <WebhooksPageUI tenantId={org} endpoints={endpoints} />
}
