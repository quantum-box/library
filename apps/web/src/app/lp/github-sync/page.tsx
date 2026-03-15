import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import {
	ArrowRight,
	CheckCircle2,
	Code2,
	FileText,
	FolderSync,
	Github,
	GitPullRequest,
	Settings,
	Sparkles,
	Zap,
} from 'lucide-react'
import Image from 'next/image'
import Link from 'next/link'

export const runtime = 'edge'

export default function GitHubSyncLandingPage() {
	return (
		<div className='min-h-screen bg-gradient-to-b from-zinc-950 via-zinc-900 to-zinc-950'>
			{/* Hero Section */}
			<section className='relative overflow-hidden'>
				<div className='absolute inset-0 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-emerald-900/20 via-transparent to-transparent' />
				<div
					className='absolute inset-0 opacity-50'
					style={{
						backgroundImage: `url("data:image/svg+xml,%3Csvg width='60' height='60' viewBox='0 0 60 60' xmlns='http://www.w3.org/2000/svg'%3E%3Cg fill='none' fill-rule='evenodd'%3E%3Cg fill='%2322c55e' fill-opacity='0.03'%3E%3Cpath d='M36 34v-4h-2v4h-4v2h4v4h2v-4h4v-2h-4zm0-30V0h-2v4h-4v2h4v4h2V6h4V4h-4zM6 34v-4H4v4H0v2h4v4h2v-4h4v-2H6zM6 4V0H4v4H0v2h4v4h2V6h4V4H6z'/%3E%3C/g%3E%3C/g%3E%3C/svg%3E")`,
					}}
				/>

				<div className='container mx-auto px-4 py-24 relative'>
					<div className='flex flex-col items-center text-center max-w-4xl mx-auto'>
						<Badge
							variant='outline'
							className='mb-6 border-emerald-500/50 text-emerald-400 bg-emerald-500/10 px-4 py-1'
						>
							<Github className='w-4 h-4 mr-2' />
							New Feature
						</Badge>

						<h1 className='text-5xl md:text-7xl font-bold text-white mb-6 tracking-tight'>
							<span className='bg-gradient-to-r from-emerald-400 via-green-400 to-teal-400 bg-clip-text text-transparent'>
								GitHub Sync
							</span>
							<br />
							<span className='text-zinc-100'>for Library</span>
						</h1>

						<p className='text-xl text-zinc-400 mb-8 max-w-2xl leading-relaxed'>
							ライブラリのデータを
							<span className='text-emerald-400 font-semibold'>
								{' '}
								Frontmatter Markdown{' '}
							</span>
							として GitHubに自動同期。
							<br />
							バージョン管理、レビュー、CI/CD との連携が簡単に。
						</p>

						<div className='flex flex-col sm:flex-row gap-4'>
							<Button
								size='lg'
								className='bg-emerald-600 hover:bg-emerald-500 text-white px-8'
								asChild
							>
								<Link href='/sign_in'>
									Get Started
									<ArrowRight className='ml-2 h-5 w-5' />
								</Link>
							</Button>
							<Button
								size='lg'
								variant='outline'
								className='border-zinc-700 text-zinc-300 hover:bg-zinc-800'
								asChild
							>
								<a
									href='https://github.com/quantum-box/tachyon-apps'
									target='_blank'
									rel='noopener noreferrer'
								>
									<Github className='mr-2 h-5 w-5' />
									View on GitHub
								</a>
							</Button>
						</div>
					</div>
				</div>
			</section>

			{/* Code Preview Section */}
			<section className='py-20 relative'>
				<div className='container mx-auto px-4'>
					<div className='max-w-4xl mx-auto'>
						<div className='bg-zinc-900 rounded-xl border border-zinc-800 overflow-hidden shadow-2xl'>
							<div className='flex items-center gap-2 px-4 py-3 bg-zinc-800/50 border-b border-zinc-700'>
								<div className='flex gap-2'>
									<div className='w-3 h-3 rounded-full bg-red-500' />
									<div className='w-3 h-3 rounded-full bg-yellow-500' />
									<div className='w-3 h-3 rounded-full bg-green-500' />
								</div>
								<span className='text-zinc-400 text-sm ml-2 font-mono'>
									docs/articles/my-article.md
								</span>
							</div>
							<pre className='p-6 text-sm overflow-x-auto'>
								<code className='language-yaml'>
									<span className='text-zinc-500'>---</span>
									{'\n'}
									<span className='text-emerald-400'>id</span>
									<span className='text-zinc-400'>:</span>{' '}
									<span className='text-amber-300'>data_01abc123xyz</span>
									{'\n'}
									<span className='text-emerald-400'>title</span>
									<span className='text-zinc-400'>:</span>{' '}
									<span className='text-amber-300'>
										&quot;Getting Started with Library&quot;
									</span>
									{'\n'}
									<span className='text-emerald-400'>tags</span>
									<span className='text-zinc-400'>:</span>
									{'\n'}
									<span className='text-zinc-400'> -</span>{' '}
									<span className='text-amber-300'>tutorial</span>
									{'\n'}
									<span className='text-zinc-400'> -</span>{' '}
									<span className='text-amber-300'>documentation</span>
									{'\n'}
									<span className='text-emerald-400'>ext_github</span>
									<span className='text-zinc-400'>:</span>
									{'\n'}
									<span className='text-zinc-400'> </span>
									<span className='text-cyan-400'>repo</span>
									<span className='text-zinc-400'>:</span>{' '}
									<span className='text-amber-300'>owner/my-docs</span>
									{'\n'}
									<span className='text-zinc-400'> </span>
									<span className='text-cyan-400'>path</span>
									<span className='text-zinc-400'>:</span>{' '}
									<span className='text-amber-300'>
										docs/articles/my-article.md
									</span>
									{'\n'}
									<span className='text-zinc-500'>---</span>
									{'\n\n'}
									<span className='text-zinc-300'>
										# Getting Started with Library
									</span>
									{'\n\n'}
									<span className='text-zinc-400'>
										Your content here, synced automatically...
									</span>
								</code>
							</pre>
						</div>
					</div>
				</div>
			</section>

			{/* Screenshots Section */}
			<section className='py-20 bg-zinc-900/50'>
				<div className='container mx-auto px-4'>
					<div className='text-center mb-16'>
						<h2 className='text-3xl md:text-4xl font-bold text-white mb-4'>
							Simple Configuration
						</h2>
						<p className='text-zinc-400 text-lg'>
							トグル一つで有効化、直感的な設定画面
						</p>
					</div>

					<div className='max-w-5xl mx-auto grid md:grid-cols-2 gap-8'>
						<div className='space-y-4'>
							<h3 className='text-xl font-semibold text-white'>
								1. Settings で有効化
							</h3>
							<p className='text-zinc-400'>
								Integrations タブで GitHub Sync
								トグルをオンにするだけ。複雑な設定は不要です。
							</p>
							<div className='rounded-xl overflow-hidden border border-zinc-700 shadow-2xl'>
								<Image
									src='/screenshots/github-sync-settings-on.png'
									alt='GitHub Sync Settings'
									width={800}
									height={600}
									className='w-full'
								/>
							</div>
						</div>

						<div className='space-y-4'>
							<h3 className='text-xl font-semibold text-white'>
								2. 無効時の説明
							</h3>
							<p className='text-zinc-400'>
								機能の詳細な説明とステップバイステップのガイドが表示されます。
							</p>
							<div className='rounded-xl overflow-hidden border border-zinc-700 shadow-2xl'>
								<Image
									src='/screenshots/github-sync-settings-off.png'
									alt='GitHub Sync Disabled'
									width={800}
									height={600}
									className='w-full'
								/>
							</div>
						</div>
					</div>
				</div>
			</section>

			{/* Features Grid */}
			<section className='py-20'>
				<div className='container mx-auto px-4'>
					<div className='text-center mb-16'>
						<h2 className='text-3xl md:text-4xl font-bold text-white mb-4'>
							Developer-First Features
						</h2>
						<p className='text-zinc-400 text-lg max-w-2xl mx-auto'>
							開発者のワークフローに最適化された機能セット
						</p>
					</div>

					<div className='grid md:grid-cols-2 lg:grid-cols-3 gap-6 max-w-6xl mx-auto'>
						<FeatureCard
							icon={<FolderSync className='h-6 w-6' />}
							title='自動同期'
							description='データ更新時に自動でGitHubへプッシュ。手動操作不要で常に最新状態を維持。'
						/>
						<FeatureCard
							icon={<FileText className='h-6 w-6' />}
							title='Frontmatter Markdown'
							description='メタデータをYAML frontmatterとして出力。静的サイトジェネレーターとの連携も簡単。'
						/>
						<FeatureCard
							icon={<GitPullRequest className='h-6 w-6' />}
							title='バージョン管理'
							description='GitHubの強力なバージョン管理機能をフル活用。差分確認、履歴追跡が可能に。'
						/>
						<FeatureCard
							icon={<Settings className='h-6 w-6' />}
							title='トグルで簡単設定'
							description='Settings画面のトグル一つでGitHub Syncを有効化。複雑な設定は不要。'
						/>
						<FeatureCard
							icon={<Code2 className='h-6 w-6' />}
							title='GraphQL API'
							description='プログラマブルなGraphQL APIで柔軟な連携。CI/CDパイプラインとの統合も。'
						/>
						<FeatureCard
							icon={<Zap className='h-6 w-6' />}
							title='一括同期'
							description='既存データを一括でGitHubに同期。移行作業も簡単に完了。'
						/>
					</div>
				</div>
			</section>

			{/* How It Works */}
			<section className='py-20'>
				<div className='container mx-auto px-4'>
					<div className='text-center mb-16'>
						<h2 className='text-3xl md:text-4xl font-bold text-white mb-4'>
							How It Works
						</h2>
						<p className='text-zinc-400 text-lg'>3ステップで始められます</p>
					</div>

					<div className='max-w-4xl mx-auto'>
						<div className='space-y-8'>
							<StepCard
								number={1}
								title='GitHub連携を設定'
								description='Settings > GitHub IntegrationでGitHubアカウントを連携。OAuth認証で安全に接続。'
							/>
							<StepCard
								number={2}
								title='GitHub Syncを有効化'
								description='Settings > IntegrationsでGitHub Syncトグルをオン。ext_githubプロパティが自動作成されます。'
							/>
							<StepCard
								number={3}
								title='リポジトリを設定して同期開始'
								description='Propertiesページで同期先リポジトリとパスを設定。データ更新時に自動でGitHubへ同期されます。'
							/>
						</div>
					</div>
				</div>
			</section>

			{/* Use Cases */}
			<section className='py-20 bg-zinc-900/50'>
				<div className='container mx-auto px-4'>
					<div className='text-center mb-16'>
						<h2 className='text-3xl md:text-4xl font-bold text-white mb-4'>
							Use Cases
						</h2>
						<p className='text-zinc-400 text-lg'>こんな用途に最適</p>
					</div>

					<div className='grid md:grid-cols-2 gap-8 max-w-4xl mx-auto'>
						<UseCaseCard
							title='技術ドキュメント管理'
							items={[
								'APIドキュメントをGitHubで管理',
								'PRベースのレビューフロー',
								'Docusaurus/VitePressとの連携',
							]}
						/>
						<UseCaseCard
							title='コンテンツ管理'
							items={[
								'ブログ記事のバージョン管理',
								'複数ライターとの協働',
								'Netlify/Vercelへの自動デプロイ',
							]}
						/>
						<UseCaseCard
							title='ナレッジベース'
							items={[
								'社内Wikiの外部公開',
								'マークダウンベースの編集',
								'GitHub Actionsでの自動処理',
							]}
						/>
						<UseCaseCard
							title='設定・定義ファイル'
							items={[
								'設定ファイルの中央管理',
								'変更履歴の完全追跡',
								'CI/CDパイプラインとの統合',
							]}
						/>
					</div>
				</div>
			</section>

			{/* CTA Section */}
			<section className='py-24'>
				<div className='container mx-auto px-4'>
					<div className='max-w-3xl mx-auto text-center'>
						<Sparkles className='h-12 w-12 text-emerald-400 mx-auto mb-6' />
						<h2 className='text-3xl md:text-4xl font-bold text-white mb-6'>
							Ready to Sync?
						</h2>
						<p className='text-zinc-400 text-lg mb-8'>
							今すぐLibraryでGitHub Syncを始めましょう。
							<br />
							無料で始められます。
						</p>
						<Button
							size='lg'
							className='bg-emerald-600 hover:bg-emerald-500 text-white px-12'
							asChild
						>
							<Link href='/sign_in'>
								Start Syncing
								<ArrowRight className='ml-2 h-5 w-5' />
							</Link>
						</Button>
					</div>
				</div>
			</section>

			{/* Footer */}
			<footer className='py-8 border-t border-zinc-800'>
				<div className='container mx-auto px-4'>
					<div className='flex flex-col md:flex-row justify-between items-center gap-4'>
						<div className='text-zinc-500 text-sm'>
							© 2025 Library. All rights reserved.
						</div>
						<div className='flex gap-6'>
							<a
								href='https://github.com/quantum-box/tachyon-apps'
								className='text-zinc-400 hover:text-white transition-colors'
								target='_blank'
								rel='noopener noreferrer'
							>
								<Github className='h-5 w-5' />
							</a>
						</div>
					</div>
				</div>
			</footer>
		</div>
	)
}

function FeatureCard({
	icon,
	title,
	description,
}: {
	icon: React.ReactNode
	title: string
	description: string
}) {
	return (
		<Card className='bg-zinc-800/50 border-zinc-700 hover:border-emerald-500/50 transition-colors'>
			<CardHeader>
				<div className='h-12 w-12 rounded-lg bg-emerald-500/10 flex items-center justify-center text-emerald-400 mb-4'>
					{icon}
				</div>
				<CardTitle className='text-white'>{title}</CardTitle>
			</CardHeader>
			<CardContent>
				<CardDescription className='text-zinc-400'>
					{description}
				</CardDescription>
			</CardContent>
		</Card>
	)
}

function StepCard({
	number,
	title,
	description,
}: {
	number: number
	title: string
	description: string
}) {
	return (
		<div className='flex gap-6 items-start'>
			<div className='flex-shrink-0 h-12 w-12 rounded-full bg-emerald-500/20 border border-emerald-500/50 flex items-center justify-center'>
				<span className='text-emerald-400 font-bold text-lg'>{number}</span>
			</div>
			<div>
				<h3 className='text-xl font-semibold text-white mb-2'>{title}</h3>
				<p className='text-zinc-400'>{description}</p>
			</div>
		</div>
	)
}

function UseCaseCard({ title, items }: { title: string; items: string[] }) {
	return (
		<Card className='bg-zinc-800/50 border-zinc-700'>
			<CardHeader>
				<CardTitle className='text-white text-lg'>{title}</CardTitle>
			</CardHeader>
			<CardContent>
				<ul className='space-y-2'>
					{items.map((item, index) => (
						<li key={index} className='flex items-center gap-2 text-zinc-400'>
							<CheckCircle2 className='h-4 w-4 text-emerald-400 flex-shrink-0' />
							{item}
						</li>
					))}
				</ul>
			</CardContent>
		</Card>
	)
}
