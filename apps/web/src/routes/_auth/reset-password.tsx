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
import {
	passwordSchema,
	PASSWORD_REQUIREMENTS_MESSAGE,
} from '@/app/(auth)/password-constants'
import { resetPasswordWithToken } from '@/auth'
import { useToastWithError } from '@/lib/error-toast'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { zodResolver } from '@hookform/resolvers/zod'
import { Link, useNavigate, createFileRoute } from '@tanstack/react-router'
import { useMemo } from 'react'
import { useForm } from 'react-hook-form'
import { z } from 'zod'

type ResetPasswordValues = {
	username: string
	token: string
	password: string
	confirmPassword: string
}

export const Route = createFileRoute('/_auth/reset-password')({
	component: ResetPasswordPage,
})

function ResetPasswordPage() {
	const navigate = useNavigate()
	const { t } = useTranslation()
	const { toast, errorToast } = useToastWithError()
	const search = Route.useSearch() as {
		username?: string
		email?: string
		token?: string
		code?: string
	}

	const usernameFromQuery = useMemo(
		() => search.username ?? search.email ?? '',
		[search.username, search.email],
	)

	const tokenFromQuery = useMemo(
		() => search.token ?? search.code ?? '',
		[search.token, search.code],
	)

	const resetSchema = useMemo(
		() =>
			z
				.object({
					username: z.string().min(1, 'Please enter your username'),
					token: z.string().min(1),
					password: passwordSchema,
					confirmPassword: z
						.string()
						.min(1, 'Please confirm your password'),
				})
				.refine(data => data.password === data.confirmPassword, {
					message: t.auth.resetPassword.errors.passwordMismatch,
					path: ['confirmPassword'],
				}),
		[t.auth.resetPassword.errors.passwordMismatch],
	)

	const form = useForm<ResetPasswordValues>({
		resolver: zodResolver(resetSchema),
		reValidateMode: 'onChange',
		defaultValues: {
			username: usernameFromQuery,
			token: tokenFromQuery,
			password: '',
			confirmPassword: '',
		},
	})

	const handleSubmitReset = async (values: ResetPasswordValues) => {
		if (!values.token) {
			toast({
				variant: 'destructive',
				title: 'Invalid token',
				description: t.auth.resetPassword.errors.invalidToken,
			})
			return
		}

		try {
			await resetPasswordWithToken(values.username, values.token, values.password)
			toast({
				title: t.auth.resetPassword.success,
				description: t.auth.resetPassword.success,
			})
			navigate({ to: '/sign_in' })
		} catch (error) {
			errorToast(error)
		}
	}

	return (
		<AuthLayout
			title={t.auth.resetPassword.title}
			description={t.auth.resetPassword.description}
		>
			<Form {...form}>
				<form
					onSubmit={form.handleSubmit(handleSubmitReset)}
					className='w-full max-w-[350px] space-y-6'
				>
					<FormField
						control={form.control}
						name='username'
						render={({ field }) => (
							<FormItem>
								<FormLabel className='text-foreground'>
									{t.auth.resetPassword.usernameLabel}
								</FormLabel>
								<FormControl>
									<Input
										{...field}
										placeholder={t.auth.resetPassword.usernamePlaceholder}
										className='bg-background border-input'
										autoComplete='username'
									/>
								</FormControl>
								<FormMessage />
							</FormItem>
						)}
					/>

					<FormField
						control={form.control}
						name='token'
						render={({ field }) => (
							<FormItem>
								<FormLabel className='text-foreground'>Reset token</FormLabel>
								<FormControl>
									<Input
										{...field}
										placeholder='Reset token'
										className='bg-background border-input'
										autoComplete='one-time-code'
										disabled={Boolean(tokenFromQuery)}
									/>
								</FormControl>
								<FormMessage />
							</FormItem>
						)}
					/>

					<FormField
						control={form.control}
						name='password'
						render={({ field }) => (
							<FormItem>
								<FormLabel className='text-foreground'>
									{t.auth.resetPassword.passwordLabel}
								</FormLabel>
								<FormControl>
									<Input
										{...field}
										type='password'
										placeholder={t.auth.resetPassword.passwordPlaceholder}
										className='bg-background border-input'
										autoComplete='new-password'
									/>
								</FormControl>
								<div className='text-xs text-muted-foreground'>
									{PASSWORD_REQUIREMENTS_MESSAGE}
								</div>
								<FormMessage />
							</FormItem>
						)}
					/>

					<FormField
						control={form.control}
						name='confirmPassword'
						render={({ field }) => (
							<FormItem>
								<FormLabel className='text-foreground'>
									{t.auth.resetPassword.confirmPasswordLabel}
								</FormLabel>
								<FormControl>
									<Input
										{...field}
										type='password'
										placeholder={t.auth.resetPassword.confirmPasswordPlaceholder}
										className='bg-background border-input'
										autoComplete='new-password'
									/>
								</FormControl>
								<FormMessage />
							</FormItem>
						)}
					/>

					<Button
						type='submit'
						disabled={form.formState.isSubmitting}
						className='w-full'
					>
						{form.formState.isSubmitting
							? t.auth.resetPassword.submitting
							: t.auth.resetPassword.submit}
					</Button>

					<Button variant='link' asChild className='w-full p-0'>
						<Link to='/sign_in'>{t.auth.resetPassword.backToSignIn}</Link>
					</Button>
				</form>
			</Form>
		</AuthLayout>
	)
}
