import { Card, CardContent, CardHeader } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'

export function RepoSkeleton() {
	return (
		<div className="container mx-auto px-4 py-8">
			{/* Header */}
			<div className="mb-6 flex items-center gap-3">
				<Skeleton className="h-8 w-48" />
				<Skeleton className="h-5 w-16 rounded-full" />
			</div>
			<Skeleton className="mb-8 h-4 w-96" />

			<div className="grid grid-cols-1 gap-8 lg:grid-cols-4">
				{/* Main content - data table */}
				<div className="lg:col-span-3">
					<Card>
						<CardHeader>
							<div className="flex items-center justify-between">
								<Skeleton className="h-6 w-24" />
								<Skeleton className="h-8 w-48" />
							</div>
						</CardHeader>
						<CardContent className="space-y-3">
							{Array.from({ length: 8 }).map((_, i) => (
								<Skeleton key={i} className="h-10 w-full" />
							))}
						</CardContent>
					</Card>
				</div>

				{/* Sidebar */}
				<div className="space-y-6">
					<Card>
						<CardHeader>
							<Skeleton className="h-5 w-16" />
						</CardHeader>
						<CardContent className="space-y-2">
							<Skeleton className="h-4 w-full" />
							<Skeleton className="h-4 w-3/4" />
						</CardContent>
					</Card>
					<Card>
						<CardHeader>
							<Skeleton className="h-5 w-24" />
						</CardHeader>
						<CardContent className="flex flex-wrap gap-2">
							{Array.from({ length: 3 }).map((_, i) => (
								<Skeleton key={i} className="h-6 w-16 rounded-full" />
							))}
						</CardContent>
					</Card>
					<Card>
						<CardHeader>
							<Skeleton className="h-5 w-28" />
						</CardHeader>
						<CardContent className="space-y-2">
							{Array.from({ length: 3 }).map((_, i) => (
								<div key={i} className="flex items-center gap-2">
									<Skeleton className="h-8 w-8 rounded-full" />
									<Skeleton className="h-4 w-24" />
								</div>
							))}
						</CardContent>
					</Card>
				</div>
			</div>
		</div>
	)
}
