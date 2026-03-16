export const runtime = 'edge'

'use client'


import { AuthLayout } from '@/components/auth-layout'
import { Button } from '@/components/ui/button'
import {
	Form,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { useToastWithError } from '@/lib/error-toast'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { zodResolver } from '@hookform/resolvers/zod'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useCallback } from 'react'
import { useForm } from 'react-hook-form'
import { z } from 'zod'
import { VERIFICATION_CODE_EXPIRY_MINUTES } from '../password-constants'
import { forgotPassword } from '../sign_up/cognito-actions'

const schema = z.object({
	username: z.string().min(1, 'Username is required'),
})

type FormData = z.infer<typeof schema>

export default function ForgotPasswordPage() {
	const router = useRouter()
	const { errorToast, toast } = useToastWithError()
	const { t } = useTranslation()
	const form = useForm<FormData>({
		resolver: zodResolver(schema),
		defaultValues: {
			username: '',
		},
	})

	const onSubmit = useCallback(
		async (data: FormData) => {
			try {
				await forgotPassword({ username: data.username })
				toast({
					variant: 'success',
					title: 'Verification code sent',
					description: `Please check your email for the 6-digit code. The code expires in ${VERIFICATION_CODE_EXPIRY_MINUTES} minutes.`,
				})
				// Navigate to reset password page with username
				router.push(
					`/reset-password?username=${encodeURIComponent(data.username)}`,
				)
			} catch (error) {
				// Handle specific Cognito errors
				if (error instanceof Error) {
					const errorName = (error as { name?: string }).name
					if (errorName === 'UserNotFoundException') {
						errorToast(new Error('User not found. Please check your username.'))
						return
					}
					if (errorName === 'LimitExceededException') {
						errorToast(
							new Error(
								'Too many attempts. Please wait a moment and try again.',
							),
						)
						return
					}
				}
				errorToast(error)
			}
		},
		[router, toast, errorToast],
	)

	return (
		<AuthLayout
			title={t.auth.forgotPassword.title}
			description={t.auth.forgotPassword.description}
		>
			<Form {...form}>
				<form
					onSubmit={form.handleSubmit(onSubmit)}
					className='space-y-6 w-full max-w-[350px]'
					aria-label='Forgot password form'
				>
					<FormField
						control={form.control}
						name='username'
						render={({ field }) => (
							<FormItem>
								<FormLabel className='text-foreground'>
									{t.auth.signIn.usernameLabel}
								</FormLabel>
								<FormControl>
									<Input
										placeholder={t.auth.signIn.usernamePlaceholder}
										autoComplete='username'
										aria-describedby='username-description'
										{...field}
										className='bg-background border-input'
									/>
								</FormControl>
								<FormMessage />
							</FormItem>
						)}
					/>

					<div className='space-y-4 flex flex-col justify-center items-center'>
						<Button
							type='submit'
							className='w-full'
							disabled={form.formState.isSubmitting}
							aria-busy={form.formState.isSubmitting}
						>
							{form.formState.isSubmitting
								? t.auth.forgotPassword.submitting
								: t.auth.forgotPassword.submit}
						</Button>

						<Button variant='link' asChild>
							<Link href='/sign_in'>{t.auth.forgotPassword.backToSignIn}</Link>
						</Button>
					</div>
				</form>
			</Form>
		</AuthLayout>
	)
}
