import {
	DateValueForEditorFragment,
	ImageValueForEditorFragment,
	IntegerValueForEditorFragment,
	LocationValueForEditorFragment,
	MultiSelectValueForEditorFragment,
	PropertyDataForEditorFragment,
	PropertyDataInputData,
	PropertyForEditorFragment,
	PropertyType,
	RelationValueForEditorFragment,
	SelectValueForEditorFragment,
	StringValueForEditorFragment,
} from '@/gen/graphql'

type MarkdownValueFragment = Extract<
	PropertyDataForEditorFragment['value'],
	{ __typename?: 'MarkdownValue' }
>

type HtmlValueFragment = Extract<
	PropertyDataForEditorFragment['value'],
	{ __typename?: 'HtmlValue' }
>

// propertiesをみて、propertyDataの型を変える
export const convertPropertyData = (
	properties: PropertyForEditorFragment[],
	propertyData: PropertyDataForEditorFragment[],
): PropertyDataInputData[] => {
	return propertyData
		.map((x): PropertyDataInputData | null => {
			const property = properties.find(p => p.id === x.propertyId)
			if (!property) {
				throw new Error('Invalid property')
			}
			switch (property.typ) {
				case PropertyType.String:
					return {
						propertyId: x.propertyId,
						value: {
							string: (x.value as StringValueForEditorFragment).string,
						},
					} satisfies PropertyDataInputData
				case PropertyType.Html: {
					const htmlValue = x.value as HtmlValueFragment | undefined
					return {
						propertyId: x.propertyId,
						value: {
							html: htmlValue?.html ?? '',
						},
					} satisfies PropertyDataInputData
				}
				case PropertyType.Markdown: {
					const markdownValue = x.value as MarkdownValueFragment | undefined
					return {
						propertyId: x.propertyId,
						value: {
							markdown: markdownValue?.markdown ?? '',
						},
					} satisfies PropertyDataInputData
				}
				case PropertyType.Integer:
					return {
						propertyId: x.propertyId,
						value: {
							integer: (x.value as IntegerValueForEditorFragment).number,
						},
					} satisfies PropertyDataInputData
				case PropertyType.Relation:
					return {
						propertyId: x.propertyId,
						value: {
							relation: (x.value as RelationValueForEditorFragment).dataIds,
						},
					} satisfies PropertyDataInputData
				case PropertyType.Select: {
					const selectValue = x.value as
						| SelectValueForEditorFragment
						| undefined
					const optionId = selectValue?.optionId ?? ''
					// Skip empty select values to avoid Oneof input objects error
					if (!optionId) {
						return null
					}
					return {
						propertyId: x.propertyId,
						value: {
							select: optionId,
						},
					} satisfies PropertyDataInputData
				}
				case PropertyType.MultiSelect:
					return {
						propertyId: x.propertyId,
						value: {
							multiSelect: (x.value as MultiSelectValueForEditorFragment)
								.optionIds,
						},
					} satisfies PropertyDataInputData
				case PropertyType.Location: {
					const locationValue = x.value as
						| LocationValueForEditorFragment
						| undefined
					return {
						propertyId: x.propertyId,
						value: {
							location: {
								latitude: locationValue?.latitude ?? 0,
								longitude: locationValue?.longitude ?? 0,
							},
						},
					} satisfies PropertyDataInputData
				}
				case PropertyType.Date: {
					const dateValue = x.value as DateValueForEditorFragment | undefined
					const date = dateValue?.date ?? ''
					// Skip empty date values to avoid Oneof input objects error
					if (!date) {
						return null
					}
					return {
						propertyId: x.propertyId,
						value: {
							date,
						},
					} satisfies PropertyDataInputData
				}
				case PropertyType.Image: {
					const imageValue = x.value as ImageValueForEditorFragment | undefined
					const url = imageValue?.url ?? ''
					// Skip empty image values to avoid Oneof input objects error
					if (!url) {
						return null
					}
					return {
						propertyId: x.propertyId,
						value: {
							image: url,
						},
					} satisfies PropertyDataInputData
				}
				default:
					throw new Error('Invalid property type')
			}
		})
		.filter((x): x is PropertyDataInputData => {
			return x !== null
		})
}
