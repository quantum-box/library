export const runtime = 'edge'

import { auth } from '@/app/(auth)/auth'
import { getBaseUrl } from '@/app/v1beta/_lib/get-base-url'
import { ErrorCode, platformAction } from '@/app/v1beta/_lib/platform-action'
import type { Metadata } from 'next'
import { notFound } from 'next/navigation'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { updateOrgAction } from './_components/action'
import { ApiKeyListServer } from './_components/api-key-list-server'
import { OrganizationPageUi } from './_components/organization-page-ui'


const ConnectionsQuery = graphql(`
  query GetConnectionsForOrg($tenantId: String!) {
    connections(tenantId: $tenantId) {
      id
      provider
      status
    }
  }
`)

type Connection = {
	id: string
	provider: string
	status: string
}

type ConnectionsResult = {
	connections?: Connection[]
}

type Props = {
	params: { org: string }
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
	const { org } = params

	try {
		const { organization } = await platformAction(
			async sdk => sdk.orgOgpMeta({ username: org }),
			{
				onError: error => {
					console.warn(
						'OGP metadata generation failed for organization:',
						org,
						error,
					)
				},
				allowAnonymous: true,
			},
		)

		const title = `${organization.name || org} | Library`
		const description =
			organization.description || `${organization.name || org} on Library`
		const repoCount = organization.repos?.length || 0
		const memberCount = organization.users?.length || 0

		const ogImageUrl = new URL(`/api/${org}/og`, getBaseUrl())
		ogImageUrl.searchParams.set('name', organization.name || org)
		if (organization.description) {
			ogImageUrl.searchParams.set('description', organization.description)
		}
		ogImageUrl.searchParams.set('repos', String(repoCount))
		ogImageUrl.searchParams.set('members', String(memberCount))

		return {
			title,
			description,
			openGraph: {
				title,
				description,
				type: 'profile',
				images: [
					{
						url: ogImageUrl.toString(),
						width: 1200,
						height: 630,
						alt: title,
					},
				],
			},
			twitter: {
				card: 'summary_large_image',
				title,
				description,
				images: [ogImageUrl.toString()],
			},
		}
	} catch {
		// Fallback: still generate OG image URL even without API data
		const ogImageUrl = new URL(`/api/${org}/og`, getBaseUrl())
		ogImageUrl.searchParams.set('name', org)

		return {
			title: `${org} | Library`,
			openGraph: {
				title: `${org} | Library`,
				type: 'profile',
				images: [
					{
						url: ogImageUrl.toString(),
						width: 1200,
						height: 630,
						alt: `${org} | Library`,
					},
				],
			},
			twitter: {
				card: 'summary_large_image',
				title: `${org} | Library`,
				images: [ogImageUrl.toString()],
			},
		}
	}
}

export default async function OrganizationPage({
	searchParams: { tab: activeTab = 'repositories' },
	params: { org },
}: {
	searchParams: {
		tab?: string
	}
	params: {
		org: string
	}
}) {
	const session = await auth()
	const { organization } = await platformAction(
		async sdk => sdk.orgPage({ username: org }),
		{
			onError: error => {
				if (error.code === ErrorCode.NOT_FOUND_ERROR) {
					notFound()
				}
				if (error.code === ErrorCode.PERMISSION_DENIED) {
					console.warn(
						'Permission denied during org page load, continuing:',
						error.message,
					)
					return
				}
			},
			redirectOnError: false,
			allowAnonymous: true,
		},
	)
	const isViewOnly = !session

	// Check Linear connection status
	const connectionsResult = await executeGraphQL<ConnectionsResult>(
		ConnectionsQuery,
		{
			tenantId: organization.id,
		},
		{
			operatorId: organization.id,
			accessToken: session?.user?.accessToken,
		},
	)
	const hasLinearConnection = connectionsResult?.connections?.some(
		connection =>
			connection.provider === 'LINEAR' && connection.status !== 'DISCONNECTED',
	)

	return (
		<OrganizationPageUi
			org={org}
			activeTab={activeTab}
			isViewOnly={isViewOnly}
			organization={organization}
			hasLinearConnection={hasLinearConnection || false}
			tenantId={organization.id}
			onSubmit={updateOrgAction}
			apiKeyListSlot={
				!isViewOnly ? (
					<ApiKeyListServer orgUsername={organization.username} />
				) : undefined
			}
		/>
	)
}
