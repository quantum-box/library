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
import { useTheme } from 'next-themes'
import { useRouter } from 'next/navigation'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { SIGNUP_SESSION_STORAGE_KEY } from './constants'
import { SignUpFormData, schema } from './type'

export function SignUpForm({
	signUpAction,
}: {
	signUpAction: (data: SignUpFormData) => void
}) {
	const router = useRouter()
	const { errorToast, toast } = useToastWithError()
	const { theme, setTheme } = useTheme()
	const { t } = useTranslation()
	const form = useForm<SignUpFormData>({
		reValidateMode: 'onChange',
		resolver: zodResolver(schema),
		defaultValues: {
			username: '',
			email: '',
			password: '',
		},
	})

	const onSubmit = async (data: SignUpFormData) => {
		try {
			await signUpAction(data)
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
			router.push('/verify-email/otp')

			toast({
				variant: 'success',
				title: t.auth.verifyEmail.signUpSuccess,
				description: t.auth.verifyEmail.signUpSuccessDescription,
			})
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
									<FormDescription className='text-muted-foreground text-xs'>
										Password must contain at least one uppercase letter, one
										lowercase letter, one number, and one symbol character.
									</FormDescription>
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

						<div className='flex justify-between items-center'>
							<Button variant='link' asChild className='p-0'>
								<a href='/sign_in'>{t.auth.signUp.signIn}</a>
							</Button>
							<Button
								type='button'
								variant='ghost'
								size='sm'
								onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
								className='text-muted-foreground'
							>
								{theme === 'dark' ? '🌞 Light Mode' : '🌙 Dark Mode'}
							</Button>
						</div>
					</form>
				</Form>
			</div>
		</AuthLayout>
	)
}
