
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
import { Link, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { z } from 'zod'
import { SIGNUP_SESSION_STORAGE_KEY } from './constants'
import { SignUpFormData, schema } from './type'

type SignUpFormValues = SignUpFormData & { confirmPassword: string }

export function SignUpForm({
	signUpAction,
}: {
	signUpAction: (data: SignUpFormData) => Promise<void>
}) {
	const navigate = useNavigate()
	const { errorToast, toast } = useToastWithError()
	const { t } = useTranslation()
	const form = useForm<SignUpFormValues>({
		reValidateMode: 'onChange',
		resolver: zodResolver(
			z
				.object({
					username: z
						.string()
						.min(3, t.auth.signUp.errors.usernameTooShort)
						.max(40, t.auth.signUp.errors.usernameTooLong)
						.regex(/^[a-zA-Z0-9]+$/, {
							message: t.auth.signUp.errors.usernameInvalid,
						}),
					email: z.string().email(t.auth.signUp.errors.emailInvalid),
					password: z
						.string()
						.min(8, t.auth.signUp.errors.passwordTooShort)
						.regex(/^(?=.*[A-Z])/, {
							message: t.auth.signUp.errors.passwordMissingUppercase,
						})
						.regex(/^(?=.*[a-z])/, {
							message: t.auth.signUp.errors.passwordMissingLowercase,
						})
						.regex(/^(?=.*[0-9])/, {
							message: t.auth.signUp.errors.passwordMissingNumber,
						}),
					confirmPassword: z.string().min(1, {
						message: t.auth.signUp.errors.confirmPasswordRequired,
					}),
				})
				.refine(data => data.password === data.confirmPassword, {
					message: t.auth.signUp.errors.passwordMismatch,
					path: ['confirmPassword'],
				}),
		),
		defaultValues: {
			username: '',
			email: '',
			password: '',
			confirmPassword: '',
		},
	})

	const onSubmit = async (data: SignUpFormValues) => {
		try {
			await signUpAction({
				username: data.username,
				email: data.email,
				password: data.password,
			})
			if (typeof window !== 'undefined') {
				sessionStorage.setItem(
					SIGNUP_SESSION_STORAGE_KEY,
					JSON.stringify({
						username: data.username,
						email: data.email,
						password: data.password,
					}),
				)
			}

			toast({
				variant: 'success',
				title: t.auth.verifyEmail.signUpSuccess,
				description: t.auth.verifyEmail.signUpSuccessDescription,
			})
			navigate({ to: '/verify-email/otp' })
		} catch (error) {
			console.error('Sign-up error:', error)
			errorToast(error)
		}
	}

	const [showPassword, setShowPassword] = useState(false)

	return (
		<AuthLayout
			title={t.auth.signUp.title}
			description={t.auth.signUp.description}
			footer={
				<div className='mt-4 max-w-[350px] text-sm'>
					<span className='text-zinc-500'>{t.auth.signUp.agreement}</span>
				</div>
			}
		>
			<div className='w-full max-w-[350px]'>
				<Form {...form}>
					<form onSubmit={form.handleSubmit(onSubmit)} className='space-y-6'>
						<FormField
							control={form.control}
							name='username'
							render={({ field }) => (
								<FormItem>
									<FormLabel className='text-foreground'>
										{t.auth.signUp.usernameLabel}
									</FormLabel>
									<FormControl>
										<Input
											placeholder={t.auth.signUp.usernamePlaceholder}
											{...field}
											className='bg-background border-input'
										/>
									</FormControl>
									<FormMessage />
								</FormItem>
							)}
						/>
						<FormField
							control={form.control}
							name='email'
							render={({ field }) => (
								<FormItem>
									<FormLabel className='text-foreground'>
										{t.auth.signUp.emailLabel}
									</FormLabel>
									<FormControl>
										<Input
											placeholder={t.auth.signUp.emailPlaceholder}
											{...field}
											className='bg-background border-input'
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
										{t.auth.signUp.passwordLabel}
									</FormLabel>
									<FormControl>
										<div className='relative'>
											<Input
												type={showPassword ? 'text' : 'password'}
												placeholder={t.auth.signUp.passwordPlaceholder}
												{...field}
												className='bg-background border-input'
												autoComplete='new-password'
											/>
											<Button
												type='button'
												variant='ghost'
												size='sm'
												className='absolute right-0 top-0 h-full px-3'
												onClick={() => setShowPassword(!showPassword)}
											>
												{showPassword
													? t.auth.passwordVisibility.hide
													: t.auth.passwordVisibility.show}
											</Button>
										</div>
									</FormControl>
									<div className='text-xs text-muted-foreground'>
										{t.auth.signUp.passwordHelp}
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
										{t.auth.signUp.confirmPasswordLabel}
									</FormLabel>
									<FormControl>
										<Input
											type={showPassword ? 'text' : 'password'}
											placeholder={t.auth.signUp.confirmPasswordPlaceholder}
											{...field}
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
							className='w-full'
							disabled={form.formState.isSubmitting}
						>
							{form.formState.isSubmitting
								? t.auth.signUp.submitting
								: t.auth.signUp.submit}
						</Button>

						<div className='flex justify-center'>
							<Button variant='link' asChild className='p-0'>
								<Link to='/sign_in'>{t.auth.signUp.signIn}</Link>
							</Button>
						</div>
					</form>
				</Form>
			</div>
		</AuthLayout>
	)
}
