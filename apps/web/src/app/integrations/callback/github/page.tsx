import { executeGraphQL, graphql } from '@/lib/graphql'
import Link from 'next/link'
import { redirect } from 'next/navigation'


const CompleteGitHubInstallMutation = graphql(`
  mutation CompleteGithubInstall($installationId: Int!, $integrationId: String!) {
    completeGithubInstall(installationId: $installationId, integrationId: $integrationId) {
      id
      integrationId
      provider
      status
    }
  }
`)

interface PageProps {
	searchParams: Promise<{
		installation_id?: string
		state?: string
		setup_action?: string
	}>
}

export default async function GitHubCallbackPage({ searchParams }: PageProps) {
	const { installation_id, state } = await searchParams

	if (!installation_id || !state) {
		return <ErrorView error='Missing required parameters from GitHub.' />
	}

	let tenantId: string
	let integrationId: string
	try {
		const decoded = Buffer.from(state, 'hex').toString('utf-8')
		const parts = decoded.split(':')
		if (parts.length < 2 || !parts[0] || !parts[1])
			throw new Error('Invalid state')
		tenantId = parts[0]
		integrationId = parts[1]
	} catch {
		return <ErrorView error='Invalid state parameter.' />
	}

	try {
		await executeGraphQL(
			CompleteGitHubInstallMutation,
			{
				installationId: Number(installation_id),
				integrationId,
			},
			{ operatorId: tenantId },
		)
	} catch (err) {
		console.error('Failed to complete GitHub installation:', err)
		return (
			<ErrorView error='Failed to complete GitHub connection. Please try again.' />
		)
	}

	redirect(`/v1beta/${tenantId}/integrations`)
}

function ErrorView({ error }: { error: string }) {
	return (
		<div className='flex min-h-screen items-center justify-center'>
			<div className='text-center'>
				<p className='text-destructive text-lg font-medium'>{error}</p>
				<Link
					href='/'
					className='mt-4 block text-sm text-muted-foreground underline'
				>
					Go to home
				</Link>
			</div>
		</div>
	)
}
