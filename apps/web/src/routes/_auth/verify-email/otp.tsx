import {
	confirmSignUpWithCode,
	resendSignUpConfirmationCode,
} from '@/auth'
import { SIGNUP_SESSION_STORAGE_KEY } from '@/app/(auth)/sign_up/constants'
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
import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { useForm } from 'react-hook-form'
import { z } from 'zod'

const storedSignUpData = () => {
	if (typeof window === 'undefined') {
		return null
	}

	const raw = sessionStorage.getItem(SIGNUP_SESSION_STORAGE_KEY)
	if (!raw) {
		return null
	}

	try {
		return JSON.parse(raw) as { username?: string; email?: string }
	} catch {
		return null
	}
}

type VerifyEmailValues = {
	username: string
	code: string
}

export const Route = createFileRoute('/_auth/verify-email/otp')({
	component: VerifyEmailPage,
})

function VerifyEmailPage() {
	const navigate = useNavigate()
	const { t } = useTranslation()
	const { toast, errorToast } = useToastWithError()
	const [isResending, setIsResending] = useState(false)
	const search = Route.useSearch() as { username?: string; email?: string }

	const initialSignUpData = useMemo(storedSignUpData, [])
	const initialUsername = search.username ?? initialSignUpData?.username ?? ''
	const email = search.email ?? initialSignUpData?.email ?? ''

	const form = useForm<VerifyEmailValues>({
		resolver: zodResolver(
			z.object({
				username: z.string().min(1, t.auth.verifyEmail.errors.usernameRequired),
				code: z
					.string()
					.min(6, t.auth.verifyEmail.errors.codeInvalidLength)
					.max(6, t.auth.verifyEmail.errors.codeInvalidLength)
					.regex(/^\d{6}$/, t.auth.verifyEmail.errors.codeInvalidLength),
			}),
		),
		reValidateMode: 'onChange',
		defaultValues: {
			username: initialUsername,
			code: '',
		},
	})

	const handleVerify = async (values: VerifyEmailValues) => {
		try {
			await confirmSignUpWithCode(values.username, values.code)
			if (typeof window !== 'undefined') {
				sessionStorage.removeItem(SIGNUP_SESSION_STORAGE_KEY)
			}
			toast({
				variant: 'success',
				title: t.auth.verifyEmail.success,
				description: t.auth.verifyEmail.signInAfterVerification,
			})
			navigate({ to: '/sign_in' })
		} catch (error) {
			errorToast(error)
		}
	}

	const handleResend = async () => {
		const username = form.getValues('username')
		if (!username) {
			form.setError('username', {
				message: t.auth.verifyEmail.errors.usernameRequired,
			})
			return
		}

		try {
			setIsResending(true)
			await resendSignUpConfirmationCode(username)
			toast({
				variant: 'success',
				title: t.auth.verifyEmail.resent,
				description: email
					? `${t.auth.verifyEmail.sentTo} ${email}`
					: t.auth.verifyEmail.helpText,
			})
		} catch (error) {
			errorToast(error)
		} finally {
			setIsResending(false)
		}
	}

	return (
		<AuthLayout
			title={t.auth.verifyEmail.title}
			description={t.auth.verifyEmail.description}
		>
			<Form {...form}>
				<form
					onSubmit={form.handleSubmit(handleVerify)}
					className='w-full max-w-[350px] space-y-6'
				>
					{email ? (
						<div className='rounded-md border bg-muted/40 px-3 py-2 text-sm text-muted-foreground'>
							{t.auth.verifyEmail.sentTo} {email}
						</div>
					) : null}

					<FormField
						control={form.control}
						name='username'
						render={({ field }) => (
							<FormItem>
								<FormLabel className='text-foreground'>
									{t.auth.verifyEmail.usernameLabel}
								</FormLabel>
								<FormControl>
									<Input
										{...field}
										placeholder={t.auth.verifyEmail.usernamePlaceholder}
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
						name='code'
						render={({ field }) => (
							<FormItem>
								<FormLabel className='text-foreground'>
									{t.auth.verifyEmail.codeLabel}
								</FormLabel>
								<FormControl>
									<Input
										{...field}
										placeholder={t.auth.verifyEmail.codePlaceholder}
										className='bg-background border-input tracking-[0.3em]'
										autoComplete='one-time-code'
										inputMode='numeric'
										maxLength={6}
									/>
								</FormControl>
								<FormMessage />
							</FormItem>
						)}
					/>

					<div className='text-xs text-muted-foreground'>
						{t.auth.verifyEmail.helpText}
					</div>

					<Button
						type='submit'
						disabled={form.formState.isSubmitting}
						className='w-full'
					>
						{form.formState.isSubmitting
							? t.auth.verifyEmail.submitting
							: t.auth.verifyEmail.submit}
					</Button>

					<div className='flex flex-col items-center gap-3'>
						<Button
							type='button'
							variant='link'
							className='p-0'
							disabled={isResending}
							onClick={handleResend}
						>
							{isResending
								? t.auth.verifyEmail.resending
								: t.auth.verifyEmail.resend}
						</Button>
						<Button variant='link' asChild className='p-0'>
							<Link to='/sign_up'>{t.auth.verifyEmail.startOver}</Link>
						</Button>
					</div>
				</form>
			</Form>
		</AuthLayout>
	)
}
