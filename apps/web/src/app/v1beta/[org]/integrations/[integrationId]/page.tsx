'use server'


import { authWithCheck } from '@/app/(auth)/auth'
import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { notFound } from 'next/navigation'
import { IntegrationDetailUI } from './components/integration-detail-ui'

const IntegrationsQuery = graphql(`
  query Integrations($tenantId: String!) {
    integrations {
      id
      provider
      name
      description
      icon
      category
      syncCapability
      supportedObjects
      requiresOauth
      isEnabled
      isFeatured
    }
    connections(tenantId: $tenantId) {
      id
      integrationId
      provider
      status
      externalAccountId
      externalAccountName
      connectedAt
      lastSyncedAt
      errorMessage
    }
  }
`)

interface PageProps {
	params: Promise<{ org: string; integrationId: string }>
}

export default async function IntegrationDetailPage({ params }: PageProps) {
	await authWithCheck()

	const locale = await detectLocale()
	const dictionary = getDictionary(locale)

	const { org, integrationId } = await params

	// Get organization to obtain tenant ID
	const { organization } = await platformAction(
		async sdk => sdk.orgPage({ username: org }),
		{
			onError: error => {
				if (error.code === ErrorCode.NOT_FOUND_ERROR) {
					notFound()
				}
			},
		},
	)

	const result = await executeGraphQL(IntegrationsQuery, {
		tenantId: organization.id,
	})

	const integrations = result?.integrations ?? []
	const connections = result?.connections ?? []

	// Find the specific integration
	const integration = integrations.find(
		(i: { id: string }) => i.id === integrationId,
	)

	if (!integration) {
		notFound()
	}

	// Find the connection for this integration
	const connection = connections.find(
		(c: { integrationId: string }) => c.integrationId === integrationId,
	)

	return (
		<IntegrationDetailUI
			orgUsername={org}
			tenantId={organization.id}
			integration={integration}
			connection={connection ?? null}
			dictionary={dictionary.v1beta.integrations}
		/>
	)
}
