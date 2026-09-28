import { useMemo, useState } from 'react'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import { DefaultRole, OrgRole } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { AlertCircle, Crown, Search, Shield, ShieldCheck } from 'lucide-react'
import { changeOrganizationMemberRoleAction } from './organization-access-actions'

interface OrganizationAccessMember {
	id: string
	name?: string | null
	email?: string | null
	image?: string | null
	role: DefaultRole
}

function toOrganizationRole(role: DefaultRole): OrgRole {
	switch (role) {
		case DefaultRole.Owner:
			return OrgRole.Owner
		case DefaultRole.Manager:
			return OrgRole.Manager
		default:
			return OrgRole.General
	}
}

export function OrganizationAccessManagement({
	tenantId,
	members,
	currentUserId,
	onMemberRoleChanged,
}: {
	tenantId: string
	members: OrganizationAccessMember[]
	currentUserId?: string
	onMemberRoleChanged: (userId: string, role: DefaultRole) => void
}) {
	const { t } = useTranslation()
	const [search, setSearch] = useState('')
	const [pendingUserId, setPendingUserId] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [notice, setNotice] = useState<string | null>(null)

	const ownerCount = useMemo(
		() => members.filter(member => member.role === DefaultRole.Owner).length,
		[members],
	)
	const filteredMembers = useMemo(() => {
		const query = search.trim().toLowerCase()
		if (!query) return members
		return members.filter(member =>
			[member.name ?? '', member.email ?? '', member.id].some(value =>
				value.toLowerCase().includes(query),
			),
		)
	}, [members, search])

	const updateRole = async (member: OrganizationAccessMember, role: OrgRole) => {
		if (toOrganizationRole(member.role) === role) return
		if (
			member.role === DefaultRole.Owner &&
			role !== OrgRole.Owner &&
			ownerCount <= 1
		) {
			setError(t.v1beta.organizationAccess.lastOwnerError)
			return
		}

		setPendingUserId(member.id)
		setError(null)
		setNotice(null)
		try {
			const updated = await changeOrganizationMemberRoleAction({
				tenantId,
				userId: member.id,
				newRole: role,
			})
			onMemberRoleChanged(member.id, updated.role)
			setNotice(
				t.v1beta.organizationAccess.updated.replace(
					'{name}',
					member.name?.trim() || member.email || member.id,
				),
			)
		} catch (updateError) {
			setError(
				updateError instanceof Error
					? updateError.message
					: t.v1beta.organizationAccess.updateFailed,
			)
		} finally {
			setPendingUserId(null)
		}
	}

	const roles = [
		{
			role: OrgRole.General,
			name: t.v1beta.organizationAccess.roles.general.name,
			description: t.v1beta.organizationAccess.roles.general.description,
			icon: Shield,
		},
		{
			role: OrgRole.Manager,
			name: t.v1beta.organizationAccess.roles.manager.name,
			description: t.v1beta.organizationAccess.roles.manager.description,
			icon: ShieldCheck,
		},
		{
			role: OrgRole.Owner,
			name: t.v1beta.organizationAccess.roles.owner.name,
			description: t.v1beta.organizationAccess.roles.owner.description,
			icon: Crown,
		},
	]

	return (
		<Card>
			<CardHeader>
				<CardTitle className='flex items-center gap-2 text-base'>
					<ShieldCheck className='h-4 w-4' />
					{t.v1beta.organizationAccess.title}
				</CardTitle>
				<CardDescription>
					{t.v1beta.organizationAccess.description}
				</CardDescription>
			</CardHeader>
			<CardContent className='space-y-6'>
				<div className='grid gap-3 md:grid-cols-3'>
					{roles.map(({ role, name, description, icon: Icon }) => (
						<div key={role} className='rounded-lg border p-4'>
							<div className='mb-2 flex items-center gap-2'>
								<Icon className='h-4 w-4 text-muted-foreground' />
								<span className='text-sm font-medium'>{name}</span>
							</div>
							<p className='text-xs leading-relaxed text-muted-foreground'>
								{description}
							</p>
						</div>
					))}
				</div>

				<div className='flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between'>
					<div>
						<h3 className='text-sm font-semibold'>
							{t.v1beta.organizationAccess.membersTitle}
							<Badge variant='secondary' className='ml-2'>
								{members.length}
							</Badge>
						</h3>
						<p className='mt-1 text-xs text-muted-foreground'>
							{t.v1beta.organizationAccess.membersDescription}
						</p>
						{ownerCount === 1 && (
							<p className='mt-1 text-xs text-muted-foreground'>
								{t.v1beta.organizationAccess.lastOwnerHelp}
							</p>
						)}
					</div>
					<div className='relative sm:w-64'>
						<Search className='absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground' />
						<Input
							aria-label={t.v1beta.organizationAccess.searchMembers}
							placeholder={t.v1beta.organizationAccess.searchMembers}
							className='h-9 pl-9'
							value={search}
							onChange={event => setSearch(event.target.value)}
						/>
					</div>
				</div>

				{error && (
					<div role='alert' className='flex items-start gap-2 text-sm text-destructive'>
						<AlertCircle className='mt-0.5 h-4 w-4 shrink-0' />
						<span>{error}</span>
					</div>
				)}
				{notice && (
					<p role='status' className='text-sm text-muted-foreground'>
						{notice}
					</p>
				)}

				<div className='overflow-x-auto rounded-md border'>
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>{t.v1beta.organization.table.name}</TableHead>
								<TableHead className='hidden sm:table-cell'>
									{t.v1beta.organization.table.email}
								</TableHead>
								<TableHead className='text-right'>
									{t.v1beta.organizationAccess.roleLabel}
								</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{filteredMembers.map(member => {
								const role = toOrganizationRole(member.role)
								const isLastOwner =
									member.role === DefaultRole.Owner && ownerCount <= 1
								const displayName =
									member.name?.trim() || member.email || member.id

								return (
									<TableRow key={member.id}>
										<TableCell>
											<div className='flex min-w-48 items-center gap-3'>
												<Avatar className='h-8 w-8'>
													<AvatarImage src={member.image ?? ''} alt={displayName} />
													<AvatarFallback className='text-xs'>
														{displayName.slice(0, 1).toUpperCase()}
													</AvatarFallback>
												</Avatar>
												<div className='min-w-0'>
									<p className='truncate text-sm font-medium'>
										{displayName}
										{member.id === currentUserId && (
											<span className='ml-1.5 text-xs font-normal text-muted-foreground'>
												{t.v1beta.organizationAccess.you}
											</span>
										)}
													</p>
												</div>
											</div>
										</TableCell>
										<TableCell className='hidden text-muted-foreground sm:table-cell'>
											{member.email ?? '—'}
										</TableCell>
										<TableCell>
											<div className='flex justify-end'>
												<Select
													value={role}
													disabled={pendingUserId !== null || isLastOwner}
													onValueChange={value => {
														void updateRole(member, value as OrgRole)
													}}
												>
													<SelectTrigger
														aria-label={`${t.v1beta.organizationAccess.roleLabel}: ${displayName}`}
														className='w-36'
													>
														<SelectValue />
													</SelectTrigger>
													<SelectContent>
														{roles.map(option => (
															<SelectItem key={option.role} value={option.role}>
																{option.name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
										</TableCell>
									</TableRow>
								)
							})}
							{filteredMembers.length === 0 && (
								<TableRow>
									<TableCell
										colSpan={3}
										className='py-8 text-center text-sm text-muted-foreground'
									>
										{t.v1beta.organizationAccess.noMembersFound}
									</TableCell>
								</TableRow>
							)}
						</TableBody>
					</Table>
				</div>
			</CardContent>
		</Card>
	)
}
