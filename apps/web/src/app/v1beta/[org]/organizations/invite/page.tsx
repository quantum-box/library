'use client'

export const runtime = 'edge'

import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
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
import { ArrowLeft, Loader2, Send } from 'lucide-react'
import Link from 'next/link'
import { useParams, useRouter } from 'next/navigation'
import { useForm } from 'react-hook-form'
import * as z from 'zod'
import { inviteUser } from './action'

const inviteFormSchema = z.object({
	email: z.string().email({ message: 'Please enter a valid email address' }),
	sendNotification: z.boolean(),
})

type InviteFormValues = z.infer<typeof inviteFormSchema>

export default function InvitePage() {
	const router = useRouter()
	const { errorToast, toast } = useToastWithError()
	const { org } = useParams<{ org: string }>()
	const { t } = useTranslation()
	const form = useForm<InviteFormValues>({
		resolver: zodResolver(inviteFormSchema),
		defaultValues: {
			email: '',
			sendNotification: false,
		},
	})

	const onSubmit = async (data: InviteFormValues) => {
		try {
			await inviteUser({
				org,
				invitee: { email: data.email },
				sendNotification: data.sendNotification,
			})
			toast({
				title: t.v1beta.invite.success,
				description: `An invitation has been sent to ${data.email}`,
			})
			form.reset()
			router.push(`/v1beta/${org}`)
			// biome-ignore lint/suspicious/noExplicitAny: <explanation>
		} catch (error: any) {
			errorToast(error)
		}
	}

	return (
		<div className='container mx-auto py-10'>
			<Link
				href={`/v1beta/${org}`}
				className='inline-flex items-center text-sm text-muted-foreground hover:text-foreground mb-6'
			>
				<ArrowLeft className='mr-2 h-4 w-4' />
				{t.common.back}
			</Link>
			<Card className='max-w-md mx-auto'>
				<CardHeader>
					<CardTitle>{t.v1beta.invite.title}</CardTitle>
					<CardDescription>{t.v1beta.invite.description}</CardDescription>
				</CardHeader>
				<CardContent>
					<Form {...form}>
						<form onSubmit={form.handleSubmit(onSubmit)} className='space-y-4'>
							<FormField
								control={form.control}
								name='email'
								render={({ field }) => (
									<FormItem>
										<FormLabel>{t.v1beta.invite.emailLabel}</FormLabel>
										<FormControl>
											<Input
												placeholder={t.v1beta.invite.emailPlaceholder}
												{...field}
											/>
										</FormControl>
										<FormMessage />
									</FormItem>
								)}
							/>
							<FormField
								control={form.control}
								name='sendNotification'
								render={({ field }) => (
									<FormItem className='flex flex-row items-start space-x-3 space-y-0'>
										<FormControl>
											<Checkbox
												checked={field.value}
												onCheckedChange={field.onChange}
											/>
										</FormControl>
										<div className='space-y-1 leading-none'>
											<FormLabel>Send invitation email</FormLabel>
											<FormDescription>
												An email will be sent to the invitee with instructions
												to join.
											</FormDescription>
										</div>
									</FormItem>
								)}
							/>
							<Button
								type='submit'
								className='w-full'
								disabled={form.formState.isSubmitting}
							>
								{form.formState.isSubmitting ? (
									<Loader2 className='mr-2 h-4 w-4 animate-spin' />
								) : (
									<Send className='mr-2 h-4 w-4' />
								)}
								{form.formState.isSubmitting
									? t.v1beta.invite.inviting
									: t.v1beta.invite.inviteButton}
							</Button>
						</form>
					</Form>
				</CardContent>
			</Card>
		</div>
	)
}
