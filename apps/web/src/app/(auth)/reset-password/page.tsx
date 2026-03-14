'use client'

import { AuthLayout } from '@/components/auth-layout'
import { Button } from '@/components/ui/button'
import {
	Form,
	FormControl,
	FormDescription,
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
import { useRouter, useSearchParams } from 'next/navigation'
import { Suspense, useCallback, useState } from 'react'
import { useForm } from 'react-hook-form'
import { z } from 'zod'
import {
	PASSWORD_REQUIREMENTS_MESSAGE,
	VERIFICATION_CODE_EXPIRY_MINUTES,
	passwordSchema,
	verificationCodeSchema,
} from '../password-constants'
import { confirmForgotPassword } from '../sign_up/cognito-actions'

const schema = z
	.object({
		username: z.string().min(1, 'Username is required'),
		code: verificationCodeSchema,
		newPassword: passwordSchema,
		confirmPassword: z.string(),
	})
	.refine(data => data.newPassword === data.confirmPassword, {
		message: 'Passwords do not match',
		path: ['confirmPassword'],
	})

type FormData = z.infer<typeof schema>

function ResetPasswordForm() {
	const router = useRouter()
	const searchParams = useSearchParams()
	const { errorToast, toast } = useToastWithError()
	const [showPassword, setShowPassword] = useState(false)
	const { t } = useTranslation()

	const form = useForm<FormData>({
		resolver: zodResolver(schema),
		defaultValues: {
			username: searchParams.get('username') ?? '',
			code: '',
			newPassword: '',
			confirmPassword: '',
		},
	})

	const togglePasswordVisibility = useCallback(() => {
		setShowPassword(prev => !prev)
	}, [])

	const onSubmit = useCallback(
		async (data: FormData) => {
			try {
				await confirmForgotPassword({
					username: data.username,
					code: data.code,
					newPassword: data.newPassword,
				})
				toast({
					variant: 'success',
					title: 'Password reset successful',
					description: 'You can now sign in with your new password.',
				})
				router.push('/sign_in')
			} catch (error) {
				// Handle specific Cognito errors
				if (error instanceof Error) {
					const errorName = (error as { name?: string }).name
					if (errorName === 'CodeMismatchException') {
						errorToast(
							new Error(
								'Invalid verification code. Please check and try again.',
							),
						)
						return
					}
					if (errorName === 'ExpiredCodeException') {
						errorToast(
							new Error(
								'Verification code has expired. Please request a new code.',
							),
						)
						return
					}
					if (errorName === 'InvalidPasswordException') {
						errorToast(
							new Error(
								`Password does not meet requirements. ${PASSWORD_REQUIREMENTS_MESSAGE}`,
							),
						)
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
		<Form {...form}>
			<form
				onSubmit={form.handleSubmit(onSubmit)}
				className='space-y-6 w-full max-w-[350px]'
				aria-label='Reset password form'
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
					name='code'
					render={({ field }) => (
						<FormItem>
							<FormLabel className='text-foreground'>
								{t.auth.verifyEmail.codeLabel}
							</FormLabel>
							<FormControl>
								<Input
									placeholder={t.auth.verifyEmail.codePlaceholder}
									maxLength={6}
									inputMode='numeric'
									pattern='[0-9]*'
									autoComplete='one-time-code'
									aria-describedby='code-description'
									{...field}
									className='bg-background border-input text-center tracking-widest text-lg'
								/>
							</FormControl>
							<FormDescription id='code-description'>
								Code expires in {VERIFICATION_CODE_EXPIRY_MINUTES} minutes
							</FormDescription>
							<FormMessage />
						</FormItem>
					)}
				/>

				<FormField
					control={form.control}
					name='newPassword'
					render={({ field }) => (
						<FormItem>
							<FormLabel className='text-foreground'>
								{t.auth.resetPassword.passwordLabel}
							</FormLabel>
							<FormControl>
								<div className='relative'>
									<Input
										type={showPassword ? 'text' : 'password'}
										placeholder={t.auth.resetPassword.passwordPlaceholder}
										autoComplete='new-password'
										aria-describedby='password-requirements'
										{...field}
										className='bg-background border-input pr-16'
									/>
									<Button
										type='button'
										variant='ghost'
										size='sm'
										className='absolute right-0 top-0 h-full px-3'
										onClick={togglePasswordVisibility}
										aria-label={
											showPassword ? 'Hide password' : 'Show password'
										}
									>
										{showPassword ? 'Hide' : 'Show'}
									</Button>
								</div>
							</FormControl>
							<FormDescription id='password-requirements'>
								Min 8 characters, 1 uppercase, 1 lowercase, 1 number
							</FormDescription>
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
									type={showPassword ? 'text' : 'password'}
									placeholder={t.auth.resetPassword.confirmPasswordPlaceholder}
									autoComplete='new-password'
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
							? t.auth.resetPassword.submitting
							: t.auth.resetPassword.submit}
					</Button>

					<div className='flex gap-2'>
						<Button variant='link' asChild>
							<Link href='/forgot-password'>{t.auth.verifyEmail.resend}</Link>
						</Button>
						<Button variant='link' asChild>
							<Link href='/sign_in'>{t.auth.resetPassword.backToSignIn}</Link>
						</Button>
					</div>
				</div>
			</form>
		</Form>
	)
}

function LoadingFallback() {
	return (
		<output
			className='w-full max-w-[350px] space-y-6 animate-pulse block'
			aria-label='Loading form'
		>
			<div className='h-10 bg-muted rounded' />
			<div className='h-10 bg-muted rounded' />
			<div className='h-10 bg-muted rounded' />
			<div className='h-10 bg-muted rounded' />
			<div className='h-10 bg-primary/20 rounded' />
		</output>
	)
}

export default function ResetPasswordPage() {
	const { t } = useTranslation()

	return (
		<AuthLayout
			title={t.auth.resetPassword.title}
			description={t.auth.resetPassword.description}
		>
			<Suspense fallback={<LoadingFallback />}>
				<ResetPasswordForm />
			</Suspense>
		</AuthLayout>
	)
}
