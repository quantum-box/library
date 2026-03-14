'use client'
import { Button } from '@/components/ui/button'
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@/components/ui/card'
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
import { useToast } from '@/components/ui/use-toast'
import { getSdkPlatform } from '@/lib/apiClient'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { revalidatePathAction } from '@/lib/revalidate-action'
import { zodResolver } from '@hookform/resolvers/zod'
import type { Session } from 'next-auth'
import { useRouter } from 'next/navigation'
import { useCallback, useMemo } from 'react'
import {
	type SubmitErrorHandler,
	type SubmitHandler,
	useForm,
} from 'react-hook-form'
import { z } from 'zod'

type FormStates = {
	userName: string
}

export function NewOrgForm({
	userId,
	session,
}: {
	userId: string
	session: Session
}) {
	const router = useRouter()
	const { toast } = useToast()
	const { t } = useTranslation()

	const formStatesSchema = useMemo(
		() =>
			z.object({
				// 3-40 characters, alphanumeric, hyphen, or underscore
				userName: z
					.string()
					.min(3, {
						message: t.v1beta.newOrg.validation.minLength.replace('{min}', '3'),
					})
					.max(40, {
						message: t.v1beta.newOrg.validation.maxLength.replace(
							'{max}',
							'40',
						),
					})
					.regex(/^[a-zA-Z0-9_-]+$/, {
						message: t.v1beta.newOrg.validation.invalidFormat,
					}),
			}),
		[t],
	)

	const form = useForm<FormStates>({
		resolver: zodResolver(formStatesSchema),
		reValidateMode: 'onChange',
		defaultValues: {
			userName: '',
		},
	})

	const handleOnValid: SubmitHandler<FormStates> = useCallback(
		async formStates => {
			try {
				// create account
				if (!userId) {
					throw new Error(t.v1beta.newOrg.errors.userIdRequired)
				}
				const sdk = await getSdkPlatform(session.user.accessToken)
				await sdk.CreateOrganization({
					input: {
						name: formStates.userName,
						username: formStates.userName,
					},
				})
				// backendでのサインアップは完了する
				// ここでサインインする
				revalidatePathAction('/')
				router.push('/')
				console.log('valid')
			} catch (error) {
				console.error(error)
				if (error instanceof z.ZodError) {
					console.log(error.issues)
					toast({
						variant: 'destructive',
						title: t.v1beta.newOrg.errors.formValidation,
						description: error.issues[0].message,
					})
					return
				}
				toast({
					variant: 'destructive',
					title: t.common.error,
					description:
						(error as Error).message ?? t.v1beta.newOrg.errors.generic,
				})
			}
		},
		[router, toast, userId, t, session.user.accessToken],
	)
	const handleOnInvalid: SubmitErrorHandler<FormStates> = useCallback(
		err => {
			console.log(err)
			toast({
				variant: 'destructive',
				title: t.v1beta.newOrg.errors.formValidation,
				description: err.root?.message,
			})
		},
		[toast, t],
	)

	return (
		<div className='container max-w-5xl px-4 py-10'>
			<div className='mb-8 space-y-2'>
				<p className='text-sm text-muted-foreground'>
					{t.v1beta.organization.title}
				</p>
				<h1 className='text-2xl font-bold tracking-tight'>
					{t.v1beta.newOrg.title}
				</h1>
				<p className='text-muted-foreground max-w-2xl'>
					{t.v1beta.newOrg.description}
				</p>
			</div>

			<Form {...form}>
				<form
					onSubmit={form.handleSubmit(handleOnValid, handleOnInvalid)}
					id='new-org-form'
				>
					<div className='grid gap-6 lg:grid-cols-[1.1fr_0.9fr]'>
						<Card className='shadow-sm border-border'>
							<CardHeader>
								<CardTitle className='text-xl'>
									{t.v1beta.organization.title}
								</CardTitle>
								<CardDescription>{t.v1beta.newOrg.description}</CardDescription>
							</CardHeader>
							<CardContent className='space-y-6'>
								<FormField
									control={form.control}
									name='userName'
									render={({ field }) => (
										<FormItem>
											<FormLabel>{t.v1beta.newOrg.userName}</FormLabel>
											<FormControl>
												<Input
													{...field}
													placeholder={t.v1beta.newOrg.userNamePlaceholder}
													autoComplete='off'
												/>
											</FormControl>
											<FormDescription>
												library.dev/{field.value || 'your-team'}
											</FormDescription>
											<FormMessage />
										</FormItem>
									)}
								/>
							</CardContent>
							<CardContent className='pt-0'>
								<div className='flex flex-col gap-3 sm:flex-row sm:justify-end'>
									<Button
										variant='outline'
										type='button'
										onClick={() => router.back()}
										className='sm:w-auto'
									>
										{t.common.cancel}
									</Button>
									<Button
										type='submit'
										disabled={form.formState.isSubmitting}
										className='sm:w-auto'
									>
										{form.formState.isSubmitting
											? t.common.loading
											: t.v1beta.newOrg.createOrganization}
									</Button>
								</div>
							</CardContent>
						</Card>

						<Card className='shadow-sm border-border bg-muted/40'>
							<CardHeader>
								<CardTitle className='text-base'>
									{t.v1beta.newOrg.tips}
								</CardTitle>
								<CardDescription>
									{t.v1beta.newOrg.userNameHint}
								</CardDescription>
							</CardHeader>
							<CardContent className='space-y-3 text-sm text-muted-foreground'>
								<ul className='space-y-2 list-disc list-inside'>
									<li>{t.v1beta.newOrg.tipsList.lowercaseOnly}</li>
									<li>{t.v1beta.newOrg.tipsList.keepShort}</li>
								</ul>
							</CardContent>
						</Card>
					</div>
				</form>
			</Form>
		</div>
	)
}
