'use client'

import { Button } from '@/components/ui/button'
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
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/use-toast'
import type { OrganizationOptionFragment } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { zodResolver } from '@hookform/resolvers/zod'
import { Loader2 } from 'lucide-react'
import { useParams, useRouter } from 'next/navigation'
import { Controller, useForm } from 'react-hook-form'
import { createDatabaseAction } from './action'
import { type FormData, formSchema } from './type'

export function CreateDatabase({
	organizations,
}: {
	organizations: OrganizationOptionFragment[]
}) {
	const { org } = useParams<{ org: string }>()
	const router = useRouter()
	const { t } = useTranslation()
	const form = useForm<FormData>({
		resolver: zodResolver(formSchema),
		defaultValues: {
			name: '',
			description: '',
			primaryKey: 'default',
			isPublic: false,
		},
	})

	const onSubmit = async (data: FormData) => {
		try {
			// ここでリポジトリ作成のAPIを呼び出す
			const orgObj = organizations.find(o => o.operatorName === org)
			if (!orgObj) {
				throw new Error('Organization not found')
			}
			const id = await createDatabaseAction(data, orgObj)
			// API呼び出しが成功したと仮定
			toast({
				title: t.v1beta.newRepository.success,
				description: t.v1beta.newRepository.successDescription,
			})
		} catch (error) {
			toast({
				title: t.common.error,
				description: t.v1beta.newRepository.errorDescription,
				variant: 'destructive',
			})
		}
	}

	return (
		<div className='container max-w-5xl px-4 py-10'>
			<div className='mb-8 space-y-2'>
				<p className='text-sm text-muted-foreground'>
					{t.v1beta.repository.title}
				</p>
				<h1 className='text-2xl font-bold tracking-tight'>
					{t.v1beta.newRepository.title}
				</h1>
				<p className='text-muted-foreground max-w-2xl'>
					{t.v1beta.newRepository.description}
				</p>
			</div>

			<Form {...form}>
				<form
					onSubmit={form.handleSubmit(onSubmit)}
					className='grid gap-6 lg:grid-cols-[1.1fr_0.9fr]'
				>
					<div className='space-y-6'>
						<div className='rounded-lg border border-border bg-card shadow-sm'>
							<div className='border-b px-6 py-4'>
								<h2 className='text-lg font-semibold'>
									{t.v1beta.repository.title}
								</h2>
								<p className='text-sm text-muted-foreground'>
									{t.v1beta.newRepository.repositoryNameHint}
								</p>
							</div>
							<div className='px-6 py-6 space-y-6'>
								<FormField
									control={form.control}
									name='name'
									render={({ field }) => (
										<FormItem>
											<FormLabel>
												{t.v1beta.newRepository.repositoryName}
											</FormLabel>
											<FormControl>
												<Input
													{...field}
													placeholder={
														t.v1beta.newRepository.repositoryNamePlaceholder
													}
													autoComplete='off'
												/>
											</FormControl>
											<FormDescription>
												URL:
												<span className='font-mono px-1'>
													/{org}/{field.value || 'repository'}
												</span>
											</FormDescription>
											<FormMessage />
										</FormItem>
									)}
								/>

								<FormField
									control={form.control}
									name='description'
									render={({ field }) => (
										<FormItem>
											<FormLabel>
												{t.v1beta.newRepository.repositoryDescription}
											</FormLabel>
											<FormControl>
												<Textarea
													{...field}
													rows={4}
													placeholder={
														t.v1beta.newRepository
															.repositoryDescriptionPlaceholder
													}
												/>
											</FormControl>
											<FormMessage />
										</FormItem>
									)}
								/>
							</div>
						</div>

						<div className='rounded-lg border border-border bg-card shadow-sm'>
							<div className='border-b px-6 py-4'>
								<h2 className='text-lg font-semibold'>
									{t.v1beta.newRepository.visibility}
								</h2>
							</div>
							<div className='px-6 py-6 space-y-6'>
								<Controller
									name='primaryKey'
									control={form.control}
									render={({ field }) => (
										<div className='space-y-3'>
											<Label className='text-sm font-medium'>
												{t.v1beta.newRepository.primaryKey}
											</Label>
											<RadioGroup
												onValueChange={field.onChange}
												defaultValue={field.value}
												className='space-y-3'
											>
												<div className='flex items-start space-x-3 rounded-md border p-3 hover:bg-accent/40'>
													<RadioGroupItem value='default' id='pk-default' />
													<div>
														<Label
															htmlFor='pk-default'
															className='text-sm font-medium'
														>
															{t.v1beta.newRepository.primaryKeyDefault}
														</Label>
														<p className='text-sm text-muted-foreground'>
															{
																t.v1beta.newRepository
																	.primaryKeyDefaultDescription
															}
														</p>
													</div>
												</div>
											</RadioGroup>
										</div>
									)}
								/>

								<FormField
									control={form.control}
									name='isPublic'
									render={({ field }) => (
										<FormItem className='flex items-start space-x-3 space-y-0 rounded-md border p-3'>
											<FormControl>
												<Checkbox
													checked={field.value}
													onCheckedChange={field.onChange}
												/>
											</FormControl>
											<div className='space-y-1'>
												<FormLabel>{t.v1beta.common.public}</FormLabel>
												<FormDescription>
													{t.v1beta.newRepository.publicDescription}
												</FormDescription>
											</div>
											<FormMessage />
										</FormItem>
									)}
								/>
							</div>
						</div>
					</div>

					<div className='space-y-4'>
						<div className='rounded-lg border border-border bg-muted/40 shadow-sm p-6 text-sm text-muted-foreground'>
							<h3 className='text-base font-semibold text-foreground mb-3'>
								{t.v1beta.newRepository.tips}
							</h3>
							<ul className='list-disc list-inside space-y-2'>
								<li>{t.v1beta.newRepository.tipsList.lowercaseOnly}</li>
								<li>{t.v1beta.newRepository.tipsList.keepShort}</li>
							</ul>
						</div>

						<div className='flex flex-col sm:flex-row justify-end gap-3'>
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
								{form.formState.isSubmitting && (
									<Loader2 className='mr-2 h-4 w-4 animate-spin' />
								)}
								{t.v1beta.newRepository.createRepository}
							</Button>
						</div>
					</div>
				</form>
			</Form>
		</div>
	)
}
