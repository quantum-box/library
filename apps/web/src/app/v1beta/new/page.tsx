'use client'


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
import { NewOperatorOwnerMethod } from '@/gen/graphql'
import { getSdkPlatform, platformId } from '@/lib/apiClient'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { revalidatePathAction } from '@/lib/revalidate-action'
import { zodResolver } from '@hookform/resolvers/zod'
import type { Session } from 'next-auth'
import { useRouter } from 'next/navigation'
import { useCallback } from 'react'
import {
	type SubmitErrorHandler,
	type SubmitHandler,
	useForm,
} from 'react-hook-form'
import { z } from 'zod'

const formStatesSchema = z.object({
	// 3-40 characters, alphanumeric, hyphen, or underscore
	userName: z
		.string()
		.min(3)
		.max(40)
		.regex(/^[a-zA-Z0-9_-]+$/),
})

type FormStates = z.infer<typeof formStatesSchema>

export default function NewOrgForm({
	userId,
	session,
}: {
	userId: string
	session: Session
}) {
	const router = useRouter()
	const { toast } = useToast()
	const { t } = useTranslation()
	const {
		register,
		handleSubmit,
		formState: { errors },
	} = useForm<FormStates>({
		resolver: zodResolver(formStatesSchema),
		reValidateMode: 'onChange',
	})

	const handleOnValid: SubmitHandler<FormStates> = useCallback(
		async formStates => {
			try {
				// create account
				if (!userId) {
					throw new Error(t.v1beta.newOrg.errors.userIdRequired)
				}
				const sdk = await getSdkPlatform(session.user.accessToken)
				await sdk.createOperator({
					input: {
						platformId: platformId,
						operatorName: formStates.userName,
						operatorAlias: formStates.userName,
						newOperatorOwnerMethod: NewOperatorOwnerMethod.Inherit,
						newOperatorOwnerId: userId,
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
		<form
			onSubmit={handleSubmit(handleOnValid, handleOnInvalid)}
			id='new-org-form'
		>
			<Card className='w-full max-w-xl mx-auto my-8 p-6 bg-white shadow-md'>
				<CardHeader>
					<CardTitle>{t.v1beta.newOrg.title}</CardTitle>
					<CardDescription>{t.v1beta.newOrg.description}</CardDescription>
				</CardHeader>
				<CardContent>
					<div className='grid w-full gap-4'>
						<div>
							<Label htmlFor='user-name'>{t.v1beta.newOrg.userName}*</Label>
							<Input
								{...register('userName')}
								placeholder={t.v1beta.newOrg.userNamePlaceholder}
								className='mt-1'
							/>
							{errors.userName?.message ? (
								<p className='text-red-500'>{errors.userName.message}</p>
							) : (
								<p className='text-xs text-gray-500'>
									{t.v1beta.newOrg.userNameHint}
								</p>
							)}
						</div>
					</div>
				</CardContent>
				<CardFooter className='flex justify-end'>
					<Button type='submit' formTarget='new-org-form'>
						{t.v1beta.newOrg.createOrganization}
					</Button>
				</CardFooter>
			</Card>
		</form>
	)
}
