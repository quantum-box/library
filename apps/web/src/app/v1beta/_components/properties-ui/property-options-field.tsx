import { Button } from '@/components/ui/button'
import {
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { X } from 'lucide-react'
import { Control, useFieldArray } from 'react-hook-form'
import { PropertyFormValues } from './property-dialog'

interface PropertyOptionsFieldProps {
	control: Control<PropertyFormValues>
}

export function PropertyOptionsField({ control }: PropertyOptionsFieldProps) {
	const { fields, append, remove } = useFieldArray({
		control,
		name: 'options',
	})

	return (
		<div className='space-y-4'>
			<div>
				<FormLabel>Options</FormLabel>
				<div className='space-y-2'>
					{fields.map((field, index) => (
						<div key={`${field.id}-${index}`} className='flex gap-2'>
							<FormField
								control={control}
								name={`options.${index}.name`}
								render={({ field }) => (
									<FormItem className='flex-1'>
										<FormControl>
											<Input {...field} placeholder='Option name' />
										</FormControl>
										<FormMessage />
									</FormItem>
								)}
							/>
							<Button
								type='button'
								variant='outline'
								size='icon'
								onClick={() => remove(index)}
							>
								<X className='h-4 w-4' />
							</Button>
						</div>
					))}
				</div>
			</div>
			<Button
				type='button'
				variant='outline'
				onClick={() =>
					append({
						name: '',
						key: '',
					})
				}
			>
				Add Option
			</Button>
		</div>
	)
}
