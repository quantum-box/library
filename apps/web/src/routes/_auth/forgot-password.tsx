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
import { forgotPassword } from '@/auth'
import { useToastWithError } from '@/lib/error-toast'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { zodResolver } from '@hookform/resolvers/zod'
import { Link, useNavigate, createFileRoute } from '@tanstack/react-router'
import { useForm } from 'react-hook-form'
import { z } from 'zod'

const schema = z.object({
	email: z.string().email('Please enter a valid email address'),
})

type ForgotPasswordForm = z.infer<typeof schema>

export const Route = createFileRoute('/_auth/forgot-password')({
	component: ForgotPasswordPage,
})

function ForgotPasswordPage() {
	const navigate = useNavigate()
	const { t } = useTranslation()
	const { toast, errorToast } = useToastWithError()
	const form = useForm<ForgotPasswordForm>({
		resolver: zodResolver(schema),
		reValidateMode: 'onChange',
		defaultValues: {
			email: '',
		},
	})

	const handleSubmitForgot = async (values: ForgotPasswordForm) => {
		try {
			await forgotPassword(values.email)
			toast({
				title: t.auth.forgotPassword.success,
				description: t.auth.forgotPassword.description,
			})
			form.reset()
			navigate({ to: '/sign_in' })
		} catch (error) {
			errorToast(error)
		}
	}

	return (
		<AuthLayout
			title={t.auth.forgotPassword.title}
			description={t.auth.forgotPassword.description}
		>
			<Form {...form}>
				<form
					onSubmit={form.handleSubmit(handleSubmitForgot)}
					className='w-full max-w-[350px] space-y-6'
				>
					<FormField
						control={form.control}
						name='email'
						render={({ field }) => (
							<FormItem>
								<FormLabel className='text-foreground'>
									{t.auth.forgotPassword.emailLabel}
								</FormLabel>
								<FormControl>
									<Input
										{...field}
										placeholder={t.auth.forgotPassword.emailPlaceholder}
										className='bg-background border-input'
										autoComplete='email'
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
							? t.auth.forgotPassword.submitting
							: t.auth.forgotPassword.submit}
					</Button>

					<Button variant='link' asChild className='w-full p-0'>
						<Link to='/sign_in'>{t.auth.forgotPassword.backToSignIn}</Link>
					</Button>
				</form>
			</Form>
		</AuthLayout>
	)
}
