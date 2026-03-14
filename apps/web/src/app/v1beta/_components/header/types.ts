// セッションのユーザー型を定義
export interface SessionUser {
	user?: {
		name?: string | null
		email?: string | null
		image?: string | null
		username?: string | null
		id?: string | null
	} | null
}

// パブリックナビゲーション項目の型
export interface NavItem {
	href: string
	label: string
}

// クライアントヘッダーのプロパティ型
export interface ClientHeaderProps {
	session: SessionUser | null
	publicNavItems: NavItem[]
}
