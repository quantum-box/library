'use client'

export const runtime = 'edge'


import { Button } from '@/components/ui/button'
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
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/use-toast'
import { zodResolver } from '@hookform/resolvers/zod'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import * as z from 'zod'

const formSchema = z.object({
	name: z
		.string()
		.min(3, {
			message: 'Organization name must be at least 2 characters.',
		})
		.max(50, {
			message: 'Organization name must be at most 50 characters.',
		}),
	description: z
		.string()
		.max(500, {
			message: 'Description must not exceed 500 characters.',
		})
		.optional(),
})

export default function CreateOrganization() {
	const [isLoading, setIsLoading] = useState(false)

	const form = useForm<z.infer<typeof formSchema>>({
		resolver: zodResolver(formSchema),
		defaultValues: {
			name: '',
		},
	})

	function onSubmit(values: z.infer<typeof formSchema>) {
		setIsLoading(true)
		// Here you would call the actual API to create the organization
		console.log(values)
		setTimeout(() => {
			setIsLoading(false)
			toast({
				title: 'Organization Created',
				description: 'Your new organization has been successfully created.',
			})
			form.reset()
		}, 2000)
	}

	return (
		<div className='max-w-2xl mx-auto p-4 space-y-6'>
			<h1 className='text-2xl font-bold'>Create a New Organization</h1>
			<Form {...form}>
				<form onSubmit={form.handleSubmit(onSubmit)} className='space-y-8'>
					<FormField
						control={form.control}
						name='name'
						render={({ field }) => (
							<FormItem>
								<FormLabel>Organization Name</FormLabel>
								<FormControl>
									<Input placeholder='Enter organization name' {...field} />
								</FormControl>
								<FormDescription>
									Enter the official name of your organization.
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
								<FormLabel>Description</FormLabel>
								<FormControl>
									<Textarea
										placeholder='Enter organization description'
										className='resize-none'
										{...field}
									/>
								</FormControl>
								<FormDescription>
									Briefly describe the purpose and activities of your
									organization.
								</FormDescription>
								<FormMessage />
							</FormItem>
						)}
					/>
					<Button type='submit' disabled={isLoading}>
						{isLoading ? 'Creating...' : 'Create Organization'}
					</Button>
				</form>
			</Form>
		</div>
	)
}
