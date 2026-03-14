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
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { SignInFormData, schema } from './type'

export function SignInForm({
	signInAction,
}: {
	signInAction: (data: SignInFormData) => void
}) {
	const router = useRouter()
	const { errorToast, toast } = useToastWithError()
	const { t } = useTranslation()
	const form = useForm<SignInFormData>({
		resolver: zodResolver(schema),
		defaultValues: {
			username: '',
			password: '',
		},
	})

	const onSubmit = async (data: SignInFormData) => {
		try {
			await signInAction(data)
			router.push('/')
			toast({
				variant: 'success',
				title: 'Sign-in success',
				description: 'Welcome back!',
			})
		} catch (error) {
			console.error('Sign-in error:', error)
			errorToast(error)
		}
	}

	const [showPassword, setShowPassword] = useState(false)

	return (
		<AuthLayout
			title={t.auth.signIn.title}
			description={t.auth.signIn.description}
			footer={
				<div className='mt-4 max-w-[350px] text-sm'>
					<span className='text-zinc-500'>{t.auth.signIn.agreement}</span>
				</div>
			}
		>
			<Form {...form}>
				<form
					onSubmit={form.handleSubmit(onSubmit)}
					className='space-y-6 w-full max-w-[350px]'
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
								<div className='flex items-center justify-between'>
									<FormLabel className='text-foreground'>
										{t.auth.signIn.passwordLabel}
									</FormLabel>
									<Link
										href='/forgot-password'
										className='text-sm text-blue-500 hover:underline'
									>
										{t.auth.signIn.forgotPassword}
									</Link>
								</div>
								<FormControl>
									<div className='relative'>
										<Input
											type={showPassword ? 'text' : 'password'}
											placeholder={t.auth.signIn.passwordPlaceholder}
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

					<div className='space-y-4 flex flex-col justify-center items-center'>
						<Button
							type='submit'
							className='w-full'
							disabled={form.formState.isSubmitting}
						>
							{form.formState.isSubmitting
								? t.auth.signIn.submitting
								: t.auth.signIn.submit}
						</Button>

						<Button variant='link' asChild>
							<a href='/sign_up'>{t.auth.signIn.createAccount}</a>
						</Button>
					</div>
				</form>
			</Form>
		</AuthLayout>
	)
}
