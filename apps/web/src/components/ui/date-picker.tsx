'use client'

import { format } from 'date-fns'
import { Calendar as CalendarIcon } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Calendar } from '@/components/ui/calendar'
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover'
import { cn } from '@/lib/utils'

interface DatePickerProps {
	date?: Date
	onDateChange: (date: Date | undefined) => void
	placeholder?: string
	disabled?: boolean
}

export function DatePicker({
	date,
	onDateChange,
	placeholder = 'Pick a date',
	disabled = false,
}: DatePickerProps) {
	return (
		<Popover>
			<PopoverTrigger asChild>
				<Button
					variant='outline'
					data-empty={!date}
					disabled={disabled}
					className={cn(
						'w-[280px] justify-start text-left font-normal',
						!date && 'text-muted-foreground',
					)}
				>
					<CalendarIcon className='mr-2 h-4 w-4' />
					{date ? format(date, 'yyyy-MM-dd') : <span>{placeholder}</span>}
				</Button>
			</PopoverTrigger>
			<PopoverContent className='w-auto p-0' align='start'>
				<Calendar
					mode='single'
					selected={date}
					onSelect={onDateChange}
					initialFocus
				/>
			</PopoverContent>
		</Popover>
	)
}

