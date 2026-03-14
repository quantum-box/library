import {
	DateValueForEditorFragment,
	IntegerValueForEditorFragment,
	LocationValueForEditorFragment,
	MultiSelectValueForEditorFragment,
	PropertyDataForEditorFragment,
	PropertyForEditorFragment,
	PropertyType,
	RelationValueForEditorFragment,
	SelectValueForEditorFragment,
	StringValueForEditorFragment,
} from '@/gen/graphql'
import { LocationMap } from '../location-map'

type PropertyViewerProps = {
	propertyData: PropertyDataForEditorFragment[]
	properties: PropertyForEditorFragment[]
}

export const PropertyViewer = ({
	propertyData,
	properties,
}: PropertyViewerProps) => {
	return (
		<>
			{properties
				.filter(
					x => x.typ !== PropertyType.Html && x.typ !== PropertyType.Markdown,
				)
				.map(x => {
					switch (x.typ) {
						case PropertyType.String: {
							const stringValue = propertyData.find(y => y.propertyId === x.id)!
								.value as StringValueForEditorFragment
							return (
								<div key={x.id}>
									<div>{x.name}</div>
									<div>{stringValue.string}</div>
								</div>
							)
						}
						case PropertyType.Integer: {
							const integerVal = propertyData.find(y => x.id === y.propertyId)!
								.value as IntegerValueForEditorFragment
							return (
								<div key={x.id}>
									<div>{x.name}</div>
									<div>{integerVal.number}</div>
								</div>
							)
						}
						case PropertyType.Relation: {
							const relationVal = propertyData.find(y => x.id === y.propertyId)!
								.value as RelationValueForEditorFragment
							return (
								<div key={x.id}>
									<div>{x.name}</div>
									<div>{relationVal.dataIds}</div>
								</div>
							)
						}
						case PropertyType.Select: {
							const selectValue = propertyData.find(y => x.id === y.propertyId)!
								.value as SelectValueForEditorFragment
							return (
								<div key={x.id}>
									<div>{x.name}</div>
									<div>{selectValue.optionId}</div>
								</div>
							)
						}
						case PropertyType.MultiSelect: {
							const multiSelectValue = propertyData.find(
								y => x.id === y.propertyId,
							)!.value as MultiSelectValueForEditorFragment
							return (
								<div key={x.id}>
									<div>{x.name}</div>
									<div>{multiSelectValue.optionIds}</div>
								</div>
							)
						}
						case PropertyType.Location: {
							const locationValue = propertyData.find(
								y => x.id === y.propertyId,
							)!.value as LocationValueForEditorFragment
							return (
								<div key={x.id}>
									<div>{x.name}</div>
									<div className='mt-2'>
										<LocationMap
											latitude={locationValue.latitude}
											longitude={locationValue.longitude}
										/>
										<div className='mt-1 text-xs text-muted-foreground'>
											📍 {locationValue.latitude.toFixed(6)},{' '}
											{locationValue.longitude.toFixed(6)}
										</div>
									</div>
								</div>
							)
						}
						default: {
							return null
						}
					}
				})}
		</>
	)
}
