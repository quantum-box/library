export type DataProperty = {
	name: string
	type: 'text' | 'select' | 'number' | 'relation' | 'datetime' | 'html'
	options?: string[] // For select type
	isEssential?: boolean
	relatedDatabase?: string // For relation type
}

export interface Data {
	id: string
	name: string
	description?: string
	createdAt: string
	updatedAt: string
	[key: string]: string | number | string[] | undefined
}
