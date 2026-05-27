import { z } from 'zod'
import { passwordSchema } from '@/app/(auth)/password-constants'

export const schema = z.object({
	username: z
		.string()
		.regex(/^[a-zA-Z0-9]+$/, {
			message: 'Username must contain only alphanumeric characters',
		})
		.min(3)
		.max(40),
	email: z.string().email(),
	password: passwordSchema,
})

export type SignUpFormData = z.infer<typeof schema>
