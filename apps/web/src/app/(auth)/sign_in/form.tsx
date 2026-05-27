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
	createHostedUiAuthorizeUrl,
	getHostedUiConfig,
	type HostedUiSignInKind,
} from '@/auth/hosted-ui'
import { useToastWithError } from '@/lib/error-toast'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { zodResolver } from '@hookform/resolvers/zod'
import { Link, useNavigate } from '@tanstack/react-router'
import { KeyRound, Loader2 } from 'lucide-react'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { SignInFormData, schema } from './type'

export function SignInForm({
	signInAction,
}: {
	signInAction: (data: SignInFormData) => void
}) {
	const navigate = useNavigate()
	const { errorToast, toast } = useToastWithError()
	const { t } = useTranslation()
	const hostedUiConfig = getHostedUiConfig()
	const [hostedUiLoading, setHostedUiLoading] =
		useState<HostedUiSignInKind | null>(null)
	const [hostedUiError, setHostedUiError] = useState<string | null>(null)
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
			navigate({ to: '/' })
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

	const handleHostedUiSignIn = async (kind: HostedUiSignInKind) => {
		setHostedUiError(null)
		setHostedUiLoading(kind)

		try {
			const searchParams = new URLSearchParams(window.location.search)
			const authorizeUrl = await createHostedUiAuthorizeUrl(
				kind,
				searchParams.get('callbackUrl') ?? '/',
			)
			window.location.assign(authorizeUrl)
		} catch (error) {
			console.error('Hosted UI sign-in error:', error)
			setHostedUiLoading(null)
			setHostedUiError(t.auth.signIn.hostedUi.error)
		}
	}

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
										to='/forgot-password'
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

						<div className='w-full space-y-3'>
							<div className='relative'>
								<div className='absolute inset-0 flex items-center'>
									<span className='w-full border-t' />
								</div>
								<div className='relative flex justify-center text-xs uppercase'>
									<span className='bg-background px-2 text-muted-foreground'>
										{t.auth.signIn.hostedUi.separator}
									</span>
								</div>
							</div>

							<div className='grid w-full gap-2'>
								<Button
									type='button'
									variant='outline'
									className='w-full'
									disabled={!hostedUiConfig.ok || hostedUiLoading !== null}
									aria-label={t.auth.signIn.hostedUi.google}
									onClick={() => handleHostedUiSignIn('google')}
								>
									{hostedUiLoading === 'google' ? (
										<Loader2 className='h-4 w-4 animate-spin' />
									) : (
										<span className='inline-flex h-4 w-4 items-center justify-center rounded-full border text-[10px] font-semibold'>
											G
										</span>
									)}
									{t.auth.signIn.hostedUi.google}
								</Button>
								<Button
									type='button'
									variant='outline'
									className='w-full'
									disabled={!hostedUiConfig.ok || hostedUiLoading !== null}
									aria-label={t.auth.signIn.hostedUi.passkey}
									onClick={() => handleHostedUiSignIn('passkey')}
								>
									{hostedUiLoading === 'passkey' ? (
										<Loader2 className='h-4 w-4 animate-spin' />
									) : (
										<KeyRound className='h-4 w-4' />
									)}
									{t.auth.signIn.hostedUi.passkey}
								</Button>
							</div>

							{!hostedUiConfig.ok && (
								<p className='text-xs text-muted-foreground'>
									{t.auth.signIn.hostedUi.missingConfig}
								</p>
							)}
							{hostedUiError && (
								<p className='text-xs text-destructive'>{hostedUiError}</p>
							)}
						</div>

						<Button variant='link' asChild>
							<Link to='/sign_up'>{t.auth.signIn.createAccount}</Link>
						</Button>
					</div>
				</form>
			</Form>
		</AuthLayout>
	)
}
