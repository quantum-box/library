import { AppBar } from '@/features/LibraryAppBar'
import { Breadcrumbs } from '@/features/LibraryAppBar/breadcrump'

export function Header() {
	return (
		<AppBar>
			<Breadcrumbs />
		</AppBar>
	)
}
