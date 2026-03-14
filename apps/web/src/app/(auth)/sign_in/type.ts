import { z } from 'zod'

export const schema = z.object({
	username: z.string().min(1, { message: 'Username is required' }),
	password: z
		.string()
		.min(8, { message: 'Password must be at least 8 characters' })
		.regex(/[a-z]/, {
			message: 'Password must contain at least one lowercase letter',
		})
		.regex(/[0-9]/, { message: 'Password must contain at least one number' }),
})

export type SignInFormData = z.infer<typeof schema>
