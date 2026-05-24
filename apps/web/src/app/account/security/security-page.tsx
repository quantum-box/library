import { useSession } from '@/auth'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { AppBar } from '@/features/LibraryAppBar'
import { Link } from '@tanstack/react-router'
import { AlertCircle, Fingerprint, LinkIcon, ShieldCheck } from 'lucide-react'
import type { ComponentType } from 'react'

type SecurityCapability = {
	title: string
	description: string
	status: string
	actionLabel: string
	endpointHint: string
	icon: ComponentType<{ className?: string }>
}

const securityCapabilities: SecurityCapability[] = [
	{
		title: 'Passkeys',
		description:
			'Register a device-bound passkey for faster, phishing-resistant sign-in.',
		status: 'Coming soon: waiting for PLT-1615 WebAuthn registration APIs.',
		actionLabel: 'Register passkey',
		endpointHint:
			'Cognito StartWebAuthnRegistration / CompleteWebAuthnRegistration',
		icon: Fingerprint,
	},
	{
		title: 'Google account linking',
		description:
			'Link Google as an additional sign-in provider for this Library account.',
		status: 'Coming soon: waiting for PLT-1615 provider linking API.',
		actionLabel: 'Link Google',
		endpointHint: 'POST /auth/v1beta/link-provider',
		icon: LinkIcon,
	},
]

export function AccountSecurityPage() {
	const session = useSession()
	const account = session.data?.user

	if (session.status === 'loading') {
		return (
			<div className='min-h-screen bg-background'>
				<AppBar />
				<div className='flex min-h-[50vh] items-center justify-center'>
					<div className='h-8 w-8 animate-spin rounded-full border-b-2 border-primary' />
				</div>
			</div>
		)
	}

	if (!account) {
		return (
			<div className='min-h-screen bg-background'>
				<AppBar />
				<main className='mx-auto w-full max-w-3xl px-4 py-8'>
					<Card>
						<CardHeader>
							<CardTitle>Sign in required</CardTitle>
							<CardDescription>
								Account security settings are available after signing in.
							</CardDescription>
						</CardHeader>
						<CardContent>
							<Button asChild>
								<Link to='/sign_in'>Sign in</Link>
							</Button>
						</CardContent>
					</Card>
				</main>
			</div>
		)
	}

	return (
		<div className='min-h-screen bg-background'>
			<AppBar />
			<main className='mx-auto w-full max-w-5xl px-4 py-8'>
				<div className='space-y-6'>
					<div className='flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between'>
						<div className='space-y-1'>
							<p className='text-sm font-medium text-muted-foreground'>
								Account settings
							</p>
							<h1 className='text-2xl font-semibold tracking-tight'>
								Security
							</h1>
							<p className='max-w-2xl text-sm text-muted-foreground'>
								Prepare stronger sign-in options for this account. These
								controls are intentionally disabled until the PLT-1615 backend
								contract is available.
							</p>
						</div>
						<Badge variant='outline' className='w-fit'>
							UI skeleton
						</Badge>
					</div>

					<Alert>
						<AlertCircle className='h-4 w-4' />
						<AlertTitle>Backend dependency</AlertTitle>
						<AlertDescription>
							Passkey registration and provider linking are blocked by PLT-1615.
							This page does not call production auth APIs yet.
						</AlertDescription>
					</Alert>

					<Card>
						<CardHeader>
							<CardTitle className='flex items-center gap-2 text-base'>
								<ShieldCheck className='h-5 w-5 text-primary' />
								Account status
							</CardTitle>
							<CardDescription>
								Current Library session context for future API requests.
							</CardDescription>
						</CardHeader>
						<CardContent className='grid gap-3 sm:grid-cols-2'>
							<StatusField label='Email' value={account?.email ?? 'Unknown'} />
							<StatusField
								label='Username'
								value={account?.username ?? account?.id ?? 'Unknown'}
							/>
						</CardContent>
					</Card>

					<div className='grid gap-4 lg:grid-cols-2'>
						{securityCapabilities.map(capability => (
							<SecurityCapabilityCard
								key={capability.title}
								capability={capability}
							/>
						))}
					</div>
				</div>
			</main>
		</div>
	)
}

function StatusField({ label, value }: { label: string; value: string }) {
	return (
		<div className='rounded-md border bg-muted/20 px-3 py-2'>
			<p className='text-xs font-medium uppercase text-muted-foreground'>
				{label}
			</p>
			<p className='mt-1 truncate text-sm font-medium'>{value}</p>
		</div>
	)
}

function SecurityCapabilityCard({
	capability,
}: {
	capability: SecurityCapability
}) {
	const Icon = capability.icon

	return (
		<Card>
			<CardHeader>
				<div className='flex items-start justify-between gap-3'>
					<div className='space-y-1'>
						<CardTitle className='flex items-center gap-2 text-base'>
							<Icon className='h-5 w-5 text-primary' />
							{capability.title}
						</CardTitle>
						<CardDescription>{capability.description}</CardDescription>
					</div>
					<Badge variant='secondary'>Coming soon</Badge>
				</div>
			</CardHeader>
			<CardContent className='space-y-4'>
				<div className='rounded-md border border-dashed bg-muted/20 p-3'>
					<p className='text-sm font-medium'>Not configured</p>
					<p className='mt-1 text-sm text-muted-foreground'>
						{capability.status}
					</p>
					<p className='mt-2 text-xs text-muted-foreground'>
						Future integration: <code>{capability.endpointHint}</code>
					</p>
				</div>
				<Button type='button' disabled className='w-full sm:w-auto'>
					{capability.actionLabel}
				</Button>
			</CardContent>
		</Card>
	)
}
