import { AuthLayout } from '@/components/auth-layout'
import { signUpWithCredentials } from '@/auth'
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
import { Link, useNavigate, createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { SIGNUP_SESSION_STORAGE_KEY } from '@/app/(auth)/sign_up/constants'
import { SignUpFormData, schema } from '@/app/(auth)/sign_up/type'
import { z } from 'zod'

export const Route = createFileRoute('/_auth/sign_up')({
	component: SignUpPage,
})

function SignUpPage() {
	const navigate = useNavigate()
	const { t } = useTranslation()
	const { toast, errorToast } = useToastWithError()
	const [showPassword, setShowPassword] = useState(false)

	const form = useForm<SignUpFormData & { confirmPassword: string }>({
		resolver: zodResolver(
			z
				.object({
					username: schema.shape.username,
					email: schema.shape.email,
					password: schema.shape.password,
					confirmPassword: z.string().min(1, {
						message: 'Please confirm your password',
					}),
				})
				.refine(data => data.password === data.confirmPassword, {
					message: t.auth.signUp.errors.passwordMismatch,
					path: ['confirmPassword'],
				}),
		),
		reValidateMode: 'onChange',
		defaultValues: {
			username: '',
			email: '',
			password: '',
			confirmPassword: '',
		},
	})

	const handleSubmitSignUp = async (values: SignUpFormData & { confirmPassword: string }) => {
		try {
			await signUpWithCredentials(
				values.username,
				values.email,
				values.password,
			)

			if (typeof window !== 'undefined') {
				sessionStorage.setItem(
					SIGNUP_SESSION_STORAGE_KEY,
					JSON.stringify({
						username: values.username,
						email: values.email,
						password: values.password,
					}),
				)
			}

			toast({
				variant: 'default',
				title: t.auth.verifyEmail.signUpSuccess,
				description: t.auth.verifyEmail.signUpSuccessDescription,
			})
			navigate({ to: '/sign_in' })
		} catch (error) {
			errorToast(error)
		}
	}

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
					<form onSubmit={form.handleSubmit(handleSubmitSignUp)} className='space-y-6'>
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
											autoComplete='username'
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
											autoComplete='email'
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
											/>
											<Button
												type='button'
												variant='ghost'
												size='sm'
												className='absolute right-0 top-0 h-full px-3'
												onClick={() => setShowPassword(!showPassword)}
											>
												{showPassword ? 'Hide' : 'Show'}
											</Button>
										</div>
									</FormControl>
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

						<div className='space-y-4 flex flex-col justify-center items-center'>
							<Button
								type='submit'
								className='w-full'
								disabled={form.formState.isSubmitting}
							>
								{form.formState.isSubmitting
									? t.auth.signUp.submitting
									: t.auth.signUp.submit}
							</Button>

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
