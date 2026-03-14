'use server'

import { authWithCheck } from '@/app/(auth)/auth'
import { detectLocale } from '@/app/i18n/detect-locale'
import { getDictionary } from '@/app/i18n/get-dictionary'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { notFound } from 'next/navigation'
import { IntegrationsPageUI } from './components/integrations-page-ui'

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
	params: Promise<{ org: string }>
}

export default async function IntegrationsPage({ params }: PageProps) {
	await authWithCheck()

	const locale = await detectLocale()
	const dictionary = getDictionary(locale)

	const { org } = await params

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

	return (
		<IntegrationsPageUI
			orgUsername={org} // Organization username for URL construction
			tenantId={organization.id} // Actual tenant ID for GraphQL mutations
			integrations={integrations}
			connections={connections}
			dictionary={dictionary.v1beta.integrations}
		/>
	)
}
