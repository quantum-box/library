import { auth } from '@/app/(auth)/auth'
import Link from 'next/link'
import { ClientHeader } from './client-header'
import { NavItem, SessionUser } from './types'

export async function Header() {
	const session = await auth()

	// セッション情報をクライアントコンポーネントに渡す形式に変換
	const sessionData: SessionUser = {
		user: {
			name: session?.user?.name || null,
			email: session?.user?.email || null,
			image: session?.user?.image || null,
			username: session?.user?.username || null,
			id: session?.user?.id || null,
		},
	}

	// パブリックナビゲーション項目（未ログイン時のみ表示）
	const publicNavItems: NavItem[] = [
		{ href: '/v1beta/features', label: 'Features' },
		{ href: '/v1beta/pricing', label: 'Pricing' },
		{ href: '/v1beta/docs', label: 'Docs' },
	]

	return (
		<header className='sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60'>
			<div className='container flex h-14 items-center'>
				{/* Logo */}
				<div className='mr-4 flex items-center'>
					<Link href='/' className='mr-6 flex items-center space-x-2'>
						<span className='font-bold'>Library</span>
					</Link>
				</div>

				<ClientHeader
					session={session ? sessionData : null}
					publicNavItems={publicNavItems}
				/>
			</div>
		</header>
	)
}
