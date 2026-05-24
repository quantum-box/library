import { createFileRoute } from '@tanstack/react-router'
import { AccountSecurityPage } from '@/app/account/security/security-page'

export const Route = createFileRoute('/_auth/account/security')({
	component: AccountSecurityPage,
})
