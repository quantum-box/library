export const runtime = 'edge'

'use client'


import { AuthLayout } from '@/components/auth-layout'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useToast } from '@/components/ui/use-toast'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { Mail } from 'lucide-react'
import { signIn } from 'next-auth/react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import type { ClipboardEvent, KeyboardEvent } from 'react'
import { useEffect, useMemo, useState } from 'react'
import {
	confirmSignUpWithCognito,
	resendConfirmationCode,
} from '../../sign_up/cognito-actions'
import { SIGNUP_SESSION_STORAGE_KEY } from '../../sign_up/constants'

const CODE_LENGTH = 6
const RESEND_INTERVAL_SECONDS = 60

type SignupSessionPayload = {
	username?: string
	email?: string
	password?: string
}

export default function VerifyEmailPage() {
	const router = useRouter()
	const { toast } = useToast()
	const { t } = useTranslation()
	const [codeDigits, setCodeDigits] = useState<string[]>(
		Array.from({ length: CODE_LENGTH }, () => ''),
	)
	const [email, setEmail] = useState('')
	const [username, setUsername] = useState('')
	const [password, setPassword] = useState('')
	const [loading, setLoading] = useState(false)
	const [resending, setResending] = useState(false)
	const [timer, setTimer] = useState(RESEND_INTERVAL_SECONDS)
	const [canResend, setCanResend] = useState(false)

	useEffect(() => {
		const raw = sessionStorage.getItem(SIGNUP_SESSION_STORAGE_KEY)
		if (!raw) {
			router.replace('/sign_up')
			return
		}
		try {
			const data = JSON.parse(raw) as SignupSessionPayload
			setEmail(data.email ?? '')
			setUsername(data.username ?? '')
			setPassword(data.password ?? '')
		} catch (error) {
			console.error('Failed to parse signup session payload', error)
			router.replace('/sign_up')
		}
	}, [router])

	useEffect(() => {
		if (timer <= 0) {
			setCanResend(true)
			return
		}
		const id = window.setTimeout(() => {
			setTimer(prev => prev - 1)
		}, 1000)
		return () => window.clearTimeout(id)
	}, [timer])

	const maskedEmail = useMemo(() => {
		if (!email) return ''
		const [local, domain = ''] = email.split('@')
		if (!local) return email
		const visibleLocal = local.slice(0, 2)
		return `${visibleLocal}${'*'.repeat(Math.max(local.length - 2, 0))}@${domain}`
	}, [email])

	const handleDigitChange = (index: number, value: string) => {
		const sanitized = value.replace(/\D/g, '')
		const digits = [...codeDigits]
		if (sanitized.length > 1) {
			sanitized
				.slice(0, CODE_LENGTH)
				.split('')
				.forEach((digit, offset) => {
					const target = index + offset
					if (target < CODE_LENGTH) digits[target] = digit
				})
		} else {
			digits[index] = sanitized
		}
		setCodeDigits(digits)

		if (sanitized && index < CODE_LENGTH - 1) {
			const nextInput = document.getElementById(
				`otp-${index + 1}`,
			) as HTMLInputElement | null
			nextInput?.focus()
		}
	}

	const handleKeyDown = (
		index: number,
		event: KeyboardEvent<HTMLInputElement>,
	) => {
		if (event.key === 'Backspace' && !codeDigits[index] && index > 0) {
			const prevInput = document.getElementById(
				`otp-${index - 1}`,
			) as HTMLInputElement | null
			prevInput?.focus()
		}
	}

	const handlePaste = (
		index: number,
		event: ClipboardEvent<HTMLInputElement>,
	) => {
		const pasted = event.clipboardData.getData('text').replace(/\D/g, '')
		if (!pasted) return
		event.preventDefault()
		const digits = [...codeDigits]
		pasted
			.slice(0, CODE_LENGTH)
			.split('')
			.forEach((digit, offset) => {
				const target = index + offset
				if (target < CODE_LENGTH) digits[target] = digit
			})
		setCodeDigits(digits)
		const nextEmpty = digits.findIndex(digit => digit === '')
		const focusIndex = nextEmpty === -1 ? CODE_LENGTH - 1 : nextEmpty
		const targetInput = document.getElementById(
			`otp-${focusIndex}`,
		) as HTMLInputElement | null
		targetInput?.focus()
	}

	const resetCodeDigits = () => {
		setCodeDigits(Array.from({ length: CODE_LENGTH }, () => ''))
		const firstInput = document.getElementById(
			'otp-0',
		) as HTMLInputElement | null
		firstInput?.focus()
	}

	const handleVerify = async () => {
		const code = codeDigits.join('')
		if (code.length !== CODE_LENGTH) {
			toast({
				variant: 'destructive',
				title: t.auth.verifyEmail.codeLabel,
				description: t.auth.verifyEmail.description,
			})
			return
		}
		if (!username) {
			toast({
				variant: 'destructive',
				title: t.auth.verifyEmail.errors.failed,
				description: t.auth.verifyEmail.startOver,
			})
			router.replace('/sign_up')
			return
		}

		setLoading(true)
		try {
			await confirmSignUpWithCognito({ username, code })
			sessionStorage.removeItem(SIGNUP_SESSION_STORAGE_KEY)
			toast({
				variant: 'success',
				title: t.auth.verifyEmail.success,
			})
			if (password) {
				const result = await signIn('credentials', {
					username,
					password,
					redirect: false,
					callbackUrl: '/',
				})

				if (result?.error) {
					router.replace('/sign_in')
					return
				}

				if (result?.url) {
					router.replace(result.url)
					return
				}
			}

			router.replace('/')
		} catch (error) {
			console.error('Verification failed', error)
			const errorName = error instanceof Error ? error.name : undefined
			const description =
				errorName === 'CodeMismatchException'
					? t.auth.verifyEmail.errors.invalidCode
					: errorName === 'ExpiredCodeException'
						? t.auth.verifyEmail.errors.expired
						: t.auth.verifyEmail.errors.failed
			toast({
				variant: 'destructive',
				title: t.auth.verifyEmail.errors.failed,
				description,
			})
			resetCodeDigits()
		} finally {
			setLoading(false)
		}
	}

	const handleResend = async () => {
		if (!canResend || !username) return
		setResending(true)
		try {
			await resendConfirmationCode({ username })
			toast({
				variant: 'success',
				title: t.auth.verifyEmail.resent,
			})
			setTimer(RESEND_INTERVAL_SECONDS)
			setCanResend(false)
			resetCodeDigits()
		} catch (error) {
			console.error('Failed to resend code', error)
			toast({
				variant: 'destructive',
				title: t.auth.verifyEmail.errors.failed,
			})
		} finally {
			setResending(false)
		}
	}

	return (
		<AuthLayout title={t.auth.verifyEmail.title} description=''>
			<Card className='w-full max-w-md'>
				<CardHeader className='space-y-4 text-center'>
					<div className='mx-auto flex h-14 w-14 items-center justify-center rounded-full bg-primary/10'>
						<Mail className='h-7 w-7 text-primary' />
					</div>
					<CardTitle className='text-2xl font-semibold'>
						{t.auth.verifyEmail.title}
					</CardTitle>
					<CardDescription className='text-sm'>
						{t.auth.verifyEmail.description}
					</CardDescription>
					{maskedEmail && <p className='text-sm font-medium'>{maskedEmail}</p>}
				</CardHeader>
				<CardContent className='space-y-6'>
					<div>
						<Label className='mb-2 block text-sm font-medium'>
							{t.auth.verifyEmail.codeLabel}
						</Label>
						<div className='flex items-center justify-center gap-2'>
							{codeDigits.map((digit, index) => (
								<div className='flex flex-col items-center' key={index}>
									<Label className='sr-only'>Digit {index + 1}</Label>
									<Input
										id={`otp-${index}`}
										value={digit}
										type='text'
										inputMode='numeric'
										maxLength={1}
										onChange={event =>
											handleDigitChange(index, event.target.value)
										}
										onKeyDown={event => handleKeyDown(index, event)}
										onPaste={event => handlePaste(index, event)}
										disabled={loading}
										className='h-12 w-12 text-center text-2xl font-semibold'
										autoComplete='one-time-code'
									/>
								</div>
							))}
						</div>
					</div>
					<p className='text-center text-sm text-muted-foreground'>
						{t.auth.verifyEmail.helpText}
					</p>
				</CardContent>
				<CardFooter className='flex flex-col gap-4'>
					<Button onClick={handleVerify} className='w-full' disabled={loading}>
						{loading
							? t.auth.verifyEmail.submitting
							: t.auth.verifyEmail.submit}
					</Button>
					<Button
						variant='ghost'
						onClick={handleResend}
						disabled={!canResend || resending || loading}
					>
						{resending
							? t.auth.verifyEmail.resending
							: canResend
								? t.auth.verifyEmail.resend
								: t.auth.verifyEmail.resendCountdown.replace(
										'{seconds}',
										String(timer),
									)}
					</Button>
					<p className='text-center text-sm text-muted-foreground'>
						{t.auth.verifyEmail.wrongEmail}{' '}
						<Link href='/sign_up' className='text-primary underline'>
							{t.auth.verifyEmail.startOver}
						</Link>
					</p>
				</CardFooter>
			</Card>
		</AuthLayout>
	)
}
