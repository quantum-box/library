'use client'
import { zodResolver } from '@hookform/resolvers/zod'
import { useForm } from 'react-hook-form'
import { z } from 'zod'

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
import { Textarea } from '@/components/ui/textarea'
import { useToast } from '@/components/ui/use-toast'
import {
	OrganizationFormFragment,
	UpdateOrganizationInput,
} from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'

const formSchema = z.object({
	name: z.string().min(2, {
		message: 'Name must be at least 2 characters.',
	}),
	description: z.string().default(''),
	website: z
		.string()
		.url({ message: 'Please enter a valid URL.' })
		.or(z.literal(''))
		.default(''),
})

export function OrganizationForm({
	organization,
	onSubmit,
}: {
	organization: OrganizationFormFragment
	onSubmit: (
		val: UpdateOrganizationInput,
	) => Promise<{ id: string } | undefined>
}) {
	const { t } = useTranslation()
	const { toast } = useToast()
	const form = useForm<z.infer<typeof formSchema>>({
		resolver: zodResolver(formSchema),
		defaultValues: {
			name: organization.name,
			description: organization.description ?? '',
			website: organization.website ?? '',
		},
	})

	async function handleOnSubmit(values: z.infer<typeof formSchema>) {
		const updateOrganization = await onSubmit({
			username: organization.username,
			name: values.name,
			description: values.description,
			website: values.website || null,
		})

		if (updateOrganization) {
			toast({
				title: t.v1beta.organizationForm.success,
				description: t.v1beta.organizationForm.successDescription,
			})
		}
	}

	return (
		<Form {...form}>
			<form onSubmit={form.handleSubmit(handleOnSubmit)} className='space-y-8'>
				<FormField
					control={form.control}
					name='name'
					render={({ field }) => (
						<FormItem>
							<FormLabel>{t.v1beta.organizationForm.name}</FormLabel>
							<FormControl>
								<Input
									placeholder={t.v1beta.organizationForm.namePlaceholder}
									{...field}
								/>
							</FormControl>
							<FormMessage />
						</FormItem>
					)}
				/>
				<FormField
					control={form.control}
					name='description'
					render={({ field }) => (
						<FormItem>
							<FormLabel>{t.v1beta.organizationForm.description}</FormLabel>
							<FormControl>
								<Textarea
									placeholder={t.v1beta.organizationForm.descriptionPlaceholder}
									className='resize-none'
									{...field}
								/>
							</FormControl>
							<FormMessage />
						</FormItem>
					)}
				/>
				<FormField
					control={form.control}
					name='website'
					render={({ field }) => (
						<FormItem>
							<FormLabel>{t.v1beta.organizationForm.website}</FormLabel>
							<FormControl>
								<Input
									type='url'
									placeholder={t.v1beta.organizationForm.websitePlaceholder}
									{...field}
								/>
							</FormControl>
							<FormMessage />
						</FormItem>
					)}
				/>
				<Button type='submit'>
					{t.v1beta.organizationForm.updateOrganization}
				</Button>
			</form>
		</Form>
	)
}
