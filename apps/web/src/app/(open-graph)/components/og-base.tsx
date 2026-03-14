/**
 * Base OGP Image Component for Library
 * GitHub-style design with dark theme
 */

export interface OgStat {
	label: string
	value: string | number
	icon?: 'data' | 'contributors' | 'repos' | 'members'
}

export interface OgBadge {
	text: string
	variant: 'public' | 'private'
}

export interface OgBaseProps {
	/** Path display (e.g., "org / repo / data") */
	path?: string
	/** Main title */
	title: string
	/** Description text */
	description?: string
	/** Statistics to display */
	stats?: OgStat[]
	/** Tags/labels to display */
	tags?: string[]
	/** Visibility badge */
	badge?: OgBadge
	/** Type of OGP image */
	type: 'organization' | 'repository' | 'data'
}

// Icon components as SVG strings for ImageResponse
const icons = {
	data: (
		<svg
			width='16'
			height='16'
			viewBox='0 0 16 16'
			fill='currentColor'
			style={{ marginRight: 4 }}
		>
			<path d='M2 2.5A2.5 2.5 0 0 1 4.5 0h8.75a.75.75 0 0 1 .75.75v12.5a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 1 0-1.5h1.75v-2h-8a1 1 0 0 0-.714 1.7.75.75 0 1 1-1.072 1.05A2.495 2.495 0 0 1 2 11.5Zm10.5-1h-8a1 1 0 0 0-1 1v6.708A2.486 2.486 0 0 1 4.5 9h8ZM5 12.25a.25.25 0 0 1 .25-.25h3.5a.25.25 0 0 1 .25.25v3.25a.25.25 0 0 1-.4.2l-1.45-1.087a.249.249 0 0 0-.3 0L5.4 15.7a.25.25 0 0 1-.4-.2Z' />
		</svg>
	),
	contributors: (
		<svg
			width='16'
			height='16'
			viewBox='0 0 16 16'
			fill='currentColor'
			style={{ marginRight: 4 }}
		>
			<path d='M2 5.5a3.5 3.5 0 1 1 5.898 2.549 5.508 5.508 0 0 1 3.034 4.084.75.75 0 1 1-1.482.235 4 4 0 0 0-7.9 0 .75.75 0 0 1-1.482-.236A5.507 5.507 0 0 1 3.102 8.05 3.493 3.493 0 0 1 2 5.5ZM11 4a3.001 3.001 0 0 1 2.22 5.018 5.01 5.01 0 0 1 2.56 3.012.749.749 0 0 1-.885.954.752.752 0 0 1-.549-.514 3.507 3.507 0 0 0-2.522-2.372.75.75 0 0 1-.574-.73v-.352a.75.75 0 0 1 .416-.672A1.5 1.5 0 0 0 11 5.5.75.75 0 0 1 11 4Zm-5.5-.5a2 2 0 1 0-.001 3.999A2 2 0 0 0 5.5 3.5Z' />
		</svg>
	),
	repos: (
		<svg
			width='16'
			height='16'
			viewBox='0 0 16 16'
			fill='currentColor'
			style={{ marginRight: 4 }}
		>
			<path d='M2 2.5A2.5 2.5 0 0 1 4.5 0h8.75a.75.75 0 0 1 .75.75v12.5a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 1 0-1.5h1.75v-2h-8a1 1 0 0 0-.714 1.7.75.75 0 1 1-1.072 1.05A2.495 2.495 0 0 1 2 11.5Zm10.5-1h-8a1 1 0 0 0-1 1v6.708A2.486 2.486 0 0 1 4.5 9h8Z' />
		</svg>
	),
	members: (
		<svg
			width='16'
			height='16'
			viewBox='0 0 16 16'
			fill='currentColor'
			style={{ marginRight: 4 }}
		>
			<path d='M1.5 14.25c0 .138.112.25.25.25H4v-1.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 .75.75v1.25h2.25a.25.25 0 0 0 .25-.25V1.75a.25.25 0 0 0-.25-.25h-8.5a.25.25 0 0 0-.25.25ZM1.75 0h8.5C11.216 0 12 .784 12 1.75v12.5c0 .085-.006.168-.018.25h2.268a.75.75 0 0 1 0 1.5H.75a.75.75 0 0 1 0-1.5h.268A2.573 2.573 0 0 1 1 14.25V1.75C1 .784 1.784 0 1.75 0ZM6 3.5a.75.75 0 0 1 0 1.5H3.5a.75.75 0 0 1 0-1.5Zm0 3a.75.75 0 0 1 0 1.5H3.5a.75.75 0 0 1 0-1.5ZM3.5 10h3a.75.75 0 0 1 0 1.5h-3a.75.75 0 0 1 0-1.5Z' />
		</svg>
	),
}

/**
 * Truncate text to specified length with ellipsis
 * Safely handles non-string inputs by converting to string first
 */
function truncateText(text: unknown, maxLength: number): string {
	const str = text?.toString() || ''
	if (str.length <= maxLength) return str
	return `${str.slice(0, maxLength - 3)}...`
}

/**
 * OG Image Base Component
 * Renders a GitHub-style OGP image
 */
export function OgBase({
	path,
	title,
	description,
	stats,
	tags,
	badge,
	type,
}: OgBaseProps) {
	return (
		<div
			style={{
				display: 'flex',
				flexDirection: 'column',
				width: '100%',
				height: '100%',
				padding: '60px 80px',
				background:
					'linear-gradient(135deg, #0d1117 0%, #161b22 50%, #1c2128 100%)',
				fontFamily:
					'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
			}}
		>
			{/* Header with Logo and Badge */}
			<div
				style={{
					display: 'flex',
					alignItems: 'center',
					justifyContent: 'space-between',
					marginBottom: 24,
				}}
			>
				{/* Library Logo */}
				<div
					style={{
						display: 'flex',
						alignItems: 'center',
						gap: 12,
					}}
				>
					<svg
						width='40'
						height='40'
						viewBox='0 0 24 24'
						fill='none'
						style={{ marginRight: 8 }}
					>
						<path
							d='M4 19.5A2.5 2.5 0 0 1 6.5 17H20'
							stroke='#58a6ff'
							strokeWidth='2'
							strokeLinecap='round'
							strokeLinejoin='round'
						/>
						<path
							d='M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z'
							stroke='#58a6ff'
							strokeWidth='2'
							strokeLinecap='round'
							strokeLinejoin='round'
						/>
					</svg>
					<span
						style={{
							fontSize: 24,
							fontWeight: 600,
							color: '#58a6ff',
						}}
					>
						Library
					</span>
				</div>

				{/* Badge */}
				{badge && (
					<div
						style={{
							display: 'flex',
							alignItems: 'center',
							padding: '8px 16px',
							borderRadius: 20,
							backgroundColor:
								badge.variant === 'public' ? '#238636' : '#6e7681',
							color: '#ffffff',
							fontSize: 14,
							fontWeight: 500,
						}}
					>
						{badge.variant === 'public' ? (
							<svg
								width='16'
								height='16'
								viewBox='0 0 16 16'
								fill='currentColor'
								style={{ marginRight: 6 }}
							>
								<path d='M2 2.5A2.5 2.5 0 0 1 4.5 0h8.75a.75.75 0 0 1 .75.75v12.5a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 1 0-1.5h1.75v-2h-8a1 1 0 0 0-.714 1.7.75.75 0 1 1-1.072 1.05A2.495 2.495 0 0 1 2 11.5Zm10.5-1h-8a1 1 0 0 0-1 1v6.708A2.486 2.486 0 0 1 4.5 9h8Z' />
							</svg>
						) : (
							<svg
								width='16'
								height='16'
								viewBox='0 0 16 16'
								fill='currentColor'
								style={{ marginRight: 6 }}
							>
								<path d='M4 4a4 4 0 0 1 8 0v2h.25c.966 0 1.75.784 1.75 1.75v5.5A1.75 1.75 0 0 1 12.25 15h-8.5A1.75 1.75 0 0 1 2 13.25v-5.5C2 6.784 2.784 6 3.75 6H4Zm8.25 3.5h-8.5a.25.25 0 0 0-.25.25v5.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25v-5.5a.25.25 0 0 0-.25-.25ZM10.5 6V4a2.5 2.5 0 1 0-5 0v2Z' />
							</svg>
						)}
						{badge.text}
					</div>
				)}
			</div>

			{/* Path */}
			{path && (
				<div
					style={{
						display: 'flex',
						fontSize: 24,
						color: '#8b949e',
						marginBottom: 16,
						fontWeight: 400,
					}}
				>
					{path}
				</div>
			)}

			{/* Title */}
			<div
				style={{
					display: 'flex',
					fontSize: type === 'organization' ? 64 : 56,
					fontWeight: 700,
					color: '#ffffff',
					marginBottom: 20,
					lineHeight: 1.2,
				}}
			>
				{truncateText(title, 50)}
			</div>

			{/* Description */}
			{description && (
				<div
					style={{
						display: 'flex',
						fontSize: 24,
						color: '#c9d1d9',
						marginBottom: 24,
						lineHeight: 1.4,
						maxWidth: '90%',
					}}
				>
					{truncateText(description, 150)}
				</div>
			)}

			{/* Tags */}
			{tags && tags.length > 0 && (
				<div
					style={{
						display: 'flex',
						flexWrap: 'wrap',
						gap: 10,
						marginBottom: 24,
					}}
				>
					{tags.slice(0, 4).map((tag, index) => (
						<div
							key={index}
							style={{
								display: 'flex',
								padding: '6px 14px',
								borderRadius: 16,
								backgroundColor: '#21262d',
								color: '#58a6ff',
								fontSize: 16,
								fontWeight: 500,
								border: '1px solid #30363d',
							}}
						>
							{truncateText(tag, 20)}
						</div>
					))}
					{tags.length > 4 && (
						<div
							style={{
								display: 'flex',
								padding: '6px 14px',
								borderRadius: 16,
								backgroundColor: '#21262d',
								color: '#8b949e',
								fontSize: 16,
								fontWeight: 500,
								border: '1px solid #30363d',
							}}
						>
							+{tags.length - 4}
						</div>
					)}
				</div>
			)}

			{/* Spacer */}
			<div style={{ display: 'flex', flex: 1 }} />

			{/* Stats */}
			{stats && stats.length > 0 && (
				<div
					style={{
						display: 'flex',
						gap: 32,
						marginTop: 'auto',
					}}
				>
					{stats.map((stat, index) => (
						<div
							key={index}
							style={{
								display: 'flex',
								alignItems: 'center',
								color: '#8b949e',
								fontSize: 18,
							}}
						>
							{stat.icon && icons[stat.icon]}
							<span style={{ marginRight: 6 }}>{stat.value}</span>
							<span>{stat.label}</span>
						</div>
					))}
				</div>
			)}
		</div>
	)
}
