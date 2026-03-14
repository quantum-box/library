import { z } from 'zod'

export const schema = z.object({
	username: z
		.string()
		.regex(/^[a-zA-Z0-9]+$/, {
			message: 'Username must contain only alphanumeric characters',
		})
		.min(3)
		.max(40),
	email: z.string().email(),
	password: z
		.string()
		.min(8)
		.regex(/^(?=.*[A-Z])/, {
			message: 'Password must contain at least one uppercase letter',
		})
		.regex(/^(?=.*[a-z])/, {
			message: 'Password must contain at least one lowercase letter',
		})
		.regex(/^(?=.*[0-9])/, {
			message: 'Password must contain at least one number',
		})
		.regex(/^(?=.*[!@#$%^&*])/, {
			message: 'Password must contain at least one symbol character',
		}),
})

export type SignUpFormData = z.infer<typeof schema>
