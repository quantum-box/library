import { TableCell, TableRow } from '@/components/ui/table'
import {
	DataFieldOnRepoPageFragment,
	PropertyFieldOnRepoPageFragment,
	PropertyType,
} from '@/gen/graphql'
import NextLink from 'next/link'
import { LocationMapCompact } from '../../../_components/location-map'

export interface Props {
	data: DataFieldOnRepoPageFragment
	properties: PropertyFieldOnRepoPageFragment[]
}

export const Row = (props: Props) => {
	return (
		<TableRow>
			{props.properties.map(p => (
				<TableCell key={p.id}>
					<PropertyDisplay data={props.data} property={p} />
				</TableCell>
			))}
		</TableRow>
	)
}

const formatDate = (value?: string | null) => {
	if (!value) {
		return '—'
	}
	const date = new Date(value)
	if (Number.isNaN(date.getTime())) {
		return '—'
	}
	return new Intl.DateTimeFormat('ja-JP', {
		year: 'numeric',
		month: '2-digit',
		day: '2-digit',
	}).format(date)
}

const PropertyDisplay = ({
	data,
	property,
}: {
	data: DataFieldOnRepoPageFragment
	property: PropertyFieldOnRepoPageFragment
}) => {
	if (property.name === 'name') {
		return (
			<NextLink href={`data/${data.id}`} passHref>
				<span className='text-blue-500 hover:underline cursor-pointer'>
					{data.name}
				</span>
			</NextLink>
		)
	}
	if (property.name === 'updatedAt') {
		return formatDate(data.updatedAt)
	}
	if (property.name === 'createdAt') {
		return formatDate(data.createdAt)
	}
	switch (property.typ) {
		case PropertyType.Id:
			return (
				(
					data.propertyData.find(x => x.propertyId === property.id)?.value as {
						id?: string
					}
				)?.id ?? '—'
			)

		case PropertyType.String:
			return (
				(
					data.propertyData.find(x => x.propertyId === property.id)?.value as {
						string?: string
					}
				)?.string ?? '—'
			)

		case PropertyType.Integer:
			return (
				(
					data.propertyData.find(x => x.propertyId === property.id)?.value as {
						number?: string
					}
				)?.number ?? '—'
			)

		case PropertyType.Html:
			return 'HTML'

		case PropertyType.Select:
			return 'Select'

		case PropertyType.MultiSelect:
			return 'MultiSelect'

		case PropertyType.Location: {
			const locationVal = data.propertyData.find(
				x => x.propertyId === property.id,
			)?.value as { latitude?: number; longitude?: number } | undefined
			if (
				locationVal?.latitude !== undefined &&
				locationVal?.longitude !== undefined
			) {
				return (
					<LocationMapCompact
						latitude={locationVal.latitude}
						longitude={locationVal.longitude}
					/>
				)
			}
			return '—'
		}

		case PropertyType.Date: {
			const dateVal = data.propertyData.find(x => x.propertyId === property.id)
				?.value as { date?: string } | undefined
			return formatDate(dateVal?.date)
		}
	}

	return '—'
}
