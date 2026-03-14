import { z } from 'zod'

export const formSchema = z.object({
	name: z
		.string()
		.min(1, 'Repository name is required')
		.max(50, 'Repository name must be 50 characters or less')
		.regex(
			/^[a-zA-Z0-9_-]+$/,
			'Repository name must be alphanumeric, hyphen, or underscore',
		),
	description: z
		.string()
		.max(500, 'Description must be 500 characters or less')
		.optional(),
	primaryKey: z.enum(['auto', 'custom', 'default'], {
		required_error: 'You must select a primary key type',
	}),
	isPublic: z.boolean().optional(),
})

export type FormData = z.infer<typeof formSchema>
