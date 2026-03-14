import {
	Table,
	TableBody,
	TableHead,
	TableHeader,
	TableRow,
} from '@/components/ui/table'
import {
	DataFieldOnRepoPageFragment,
	PropertyFieldOnRepoPageFragment,
} from '@/gen/graphql'
import { Row } from './row'

interface DataTableProps {
	dataList: DataFieldOnRepoPageFragment[]
	selectedProperties: PropertyFieldOnRepoPageFragment[]
}

export function DataTable({ dataList, selectedProperties }: DataTableProps) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					{selectedProperties.map(prop => (
						<TableHead key={prop.id}>
							{prop.name.charAt(0).toUpperCase() + prop.name.slice(1)}
						</TableHead>
					))}
				</TableRow>
			</TableHeader>
			<TableBody>
				{dataList.map(item => (
					<Row key={item.id} data={item} properties={selectedProperties} />
				))}
			</TableBody>
		</Table>
	)
}
