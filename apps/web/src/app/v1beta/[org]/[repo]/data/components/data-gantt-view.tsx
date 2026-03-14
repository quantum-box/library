'use client'

import { Button } from '@/components/ui/button'
import { DatePicker } from '@/components/ui/date-picker'
import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
} from '@/components/ui/dialog'
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from '@/components/ui/select'
import { useToast } from '@/components/ui/use-toast'
import {
	DataFieldOnRepoPageFragment,
	DataForDataDetailFragment,
	PropertyDataForEditorFragment,
	PropertyFieldOnRepoPageFragment,
	PropertyForEditorFragment,
	PropertyType,
} from '@/gen/graphql'
import {
	eachDayOfInterval,
	eachMonthOfInterval,
	eachWeekOfInterval,
	endOfMonth,
	format,
	startOfMonth,
} from 'date-fns'
import { ZoomIn, ZoomOut } from 'lucide-react'
import NextLink from 'next/link'
import { useRouter } from 'next/navigation'
import { useCallback, useMemo, useRef, useState } from 'react'
import { updateData } from '../[dataId]/action'

interface DataGanttViewProps {
	data: DataFieldOnRepoPageFragment[]
	properties: PropertyFieldOnRepoPageFragment[]
	org: string
	repo: string
	canEdit?: boolean
}

interface GanttTask {
	id: string
	name: string
	startDate: Date
	endDate: Date
	dataItem: DataFieldOnRepoPageFragment
}

type ZoomLevel = 'day' | 'week' | 'month' | 'year'

export function DataGanttView({
	data,
	properties,
	org,
	repo,
	canEdit = false,
}: DataGanttViewProps) {
	const { toast } = useToast()
	const router = useRouter()
	const [editingTask, setEditingTask] = useState<GanttTask | null>(null)
	const [editStartDate, setEditStartDate] = useState<Date | undefined>()
	const [editEndDate, setEditEndDate] = useState<Date | undefined>()
	const [zoomLevel, setZoomLevel] = useState<ZoomLevel>('month')
	const [customTimelineRange, setCustomTimelineRange] = useState<{
		minDate: Date
		maxDate: Date
	} | null>(null)
	// Drag & Drop feature is temporarily disabled
	// const [draggingTask, setDraggingTask] = useState<{
	// 	taskId: string
	// 	startDate: Date
	// 	endDate: Date
	// } | null>(null)
	// const [isDragging, setIsDragging] = useState(false)
	// const dragState = useRef<{
	// 	taskId: string
	// 	startX: number
	// 	originalStartDate: Date
	// 	originalEndDate: Date
	// 	dragType: 'move' | 'resize-start' | 'resize-end'
	// 	hasMoved: boolean
	// } | null>(null)
	const timelineRef = useRef<HTMLDivElement>(null)

	// Find Date properties
	const dateProperties = useMemo(
		() => properties.filter(p => p.typ === PropertyType.Date),
		[properties],
	)

	const [startDatePropertyId, setStartDatePropertyId] = useState<string>(
		dateProperties[0]?.id || '',
	)
	const [endDatePropertyId, setEndDatePropertyId] = useState<string>(
		dateProperties[1]?.id || '',
	)

	// Convert properties to PropertyForEditorFragment format
	const editorProperties = useMemo(
		(): PropertyForEditorFragment[] =>
			properties.map(p => ({
				id: p.id,
				name: p.name,
				typ: p.typ,
				meta: p.meta as PropertyForEditorFragment['meta'],
			})),
		[properties],
	)

	// Validate date string with backend-equivalent constraints
	const validateDateString = useCallback((dateStr: string): boolean => {
		if (!dateStr || dateStr.trim() === '') {
			return false
		}

		// Check ISO 8601 format (YYYY-MM-DD)
		const dateRegex = /^\d{4}-\d{2}-\d{2}$/
		if (!dateRegex.test(dateStr)) {
			return false
		}

		const parts = dateStr.split('-')
		if (parts.length !== 3) {
			return false
		}

		const year = Number.parseInt(parts[0], 10)
		const month = Number.parseInt(parts[1], 10)
		const day = Number.parseInt(parts[2], 10)

		// Validate year range (1-9999)
		if (Number.isNaN(year) || year < 1 || year > 9999) {
			return false
		}

		// Validate month range (1-12)
		if (Number.isNaN(month) || month < 1 || month > 12) {
			return false
		}

		// Validate day range (1-31)
		if (Number.isNaN(day) || day < 1 || day > 31) {
			return false
		}

		// Validate actual date using Date object
		const date = new Date(dateStr)
		if (Number.isNaN(date.getTime())) {
			return false
		}

		// Verify the parsed date matches the input (handles invalid dates like 2024-02-30)
		const parsedYear = date.getFullYear()
		const parsedMonth = date.getMonth() + 1
		const parsedDay = date.getDate()

		if (parsedYear !== year || parsedMonth !== month || parsedDay !== day) {
			return false
		}

		return true
	}, [])

	// Convert data to Gantt tasks
	const tasks = useMemo((): GanttTask[] => {
		const invalidItems: Array<{ id: string; name: string; reason: string }> = []

		const validTasks = data
			.map(item => {
				const startDateProp = item.propertyData.find(
					pd => pd.propertyId === startDatePropertyId,
				)
				const endDateProp = item.propertyData.find(
					pd => pd.propertyId === endDatePropertyId,
				)

				const startDateValue = startDateProp?.value as
					| { date?: string }
					| undefined
				const endDateValue = endDateProp?.value as { date?: string } | undefined

				const startDate = startDateValue?.date
				const endDate = endDateValue?.date

				if (!startDate || !endDate) {
					invalidItems.push({
						id: item.id,
						name: item.name,
						reason: 'Missing start or end date',
					})
					return null
				}

				// Validate date strings with backend-equivalent constraints
				if (!validateDateString(startDate)) {
					invalidItems.push({
						id: item.id,
						name: item.name,
						reason: `Invalid start date format: ${startDate}`,
					})
					return null
				}

				if (!validateDateString(endDate)) {
					invalidItems.push({
						id: item.id,
						name: item.name,
						reason: `Invalid end date format: ${endDate}`,
					})
					return null
				}

				// Validate dates
				const start = new Date(startDate)
				const end = new Date(endDate)

				if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
					invalidItems.push({
						id: item.id,
						name: item.name,
						reason: 'Date parsing failed',
					})
					return null
				}

				if (start > end) {
					invalidItems.push({
						id: item.id,
						name: item.name,
						reason: 'Start date must be before or equal to end date',
					})
					return null
				}

				return {
					id: item.id,
					name: item.name,
					startDate: start,
					endDate: end,
					dataItem: item,
				}
			})
			.filter((task): task is GanttTask => task !== null)

		// Log invalid items for debugging
		if (invalidItems.length > 0) {
			console.warn(
				`[DataGanttView] Skipped ${invalidItems.length} items with invalid date data:`,
				invalidItems,
			)
		}

		// Show toast notification if there are invalid items
		if (invalidItems.length > 0 && validTasks.length === 0) {
			toast({
				title: 'Invalid date data',
				description:
					'All items have invalid date data. Please check the date properties.',
				variant: 'destructive',
			})
		} else if (invalidItems.length > 0) {
			toast({
				title: 'Some items skipped',
				description: `${invalidItems.length} item(s) with invalid date data were skipped.`,
				variant: 'default',
			})
		}

		return validTasks
	}, [data, startDatePropertyId, endDatePropertyId, validateDateString, toast])

	// Calculate timeline range based on zoom level
	const timelineRange = useMemo(() => {
		if (customTimelineRange) {
			return customTimelineRange
		}

		if (tasks.length === 0) return null

		const dates = tasks.flatMap(t => [t.startDate, t.endDate])
		const minDate = new Date(Math.min(...dates.map(d => d.getTime())))
		const maxDate = new Date(Math.max(...dates.map(d => d.getTime())))

		// Add padding based on zoom level
		const paddingDays =
			zoomLevel === 'day'
				? 1
				: zoomLevel === 'week'
					? 7
					: zoomLevel === 'month'
						? 30
						: 365

		minDate.setDate(minDate.getDate() - paddingDays)
		maxDate.setDate(maxDate.getDate() + paddingDays)

		return { minDate, maxDate }
	}, [tasks, zoomLevel, customTimelineRange])

	// Calculate days in range
	const daysInRange = useMemo(() => {
		if (!timelineRange) return 0
		const diffTime = Math.abs(
			timelineRange.maxDate.getTime() - timelineRange.minDate.getTime(),
		)
		const days = Math.ceil(diffTime / (1000 * 60 * 60 * 24))

		// Performance warning for large date ranges
		// TODO: Consider implementing virtualization for ranges >= 365 days
		// Virtual scrolling would improve performance by rendering only visible items
		if (days >= 365) {
			console.warn(
				`[DataGanttView] Large date range detected: ${days} days. Performance may be degraded. Consider using a smaller date range or implementing virtualization.`,
			)
		}

		return days
	}, [timelineRange])

	// Generate headers based on zoom level
	const headers = useMemo(() => {
		if (!timelineRange) return { primary: [], secondary: [] }

		switch (zoomLevel) {
			case 'day': {
				const days = eachDayOfInterval({
					start: timelineRange.minDate,
					end: timelineRange.maxDate,
				})
				return {
					primary: days.map(day => ({
						date: day,
						label: format(day, 'MMM dd'),
						width: 1,
					})),
					secondary: [],
				}
			}
			case 'week': {
				const weeks = eachWeekOfInterval(
					{
						start: timelineRange.minDate,
						end: timelineRange.maxDate,
					},
					{ weekStartsOn: 0 },
				)
				return {
					primary: weeks.map(week => ({
						date: week,
						label: format(week, 'MMM dd'),
						width: 7,
					})),
					secondary: [],
				}
			}
			case 'month': {
				const months = eachMonthOfInterval({
					start: timelineRange.minDate,
					end: timelineRange.maxDate,
				})
				const weeks = eachWeekOfInterval(
					{
						start: timelineRange.minDate,
						end: timelineRange.maxDate,
					},
					{ weekStartsOn: 0 },
				)
				return {
					primary: months.map(month => ({
						date: month,
						label: format(month, 'MMM yyyy'),
						width: Math.ceil(
							(endOfMonth(month).getTime() - startOfMonth(month).getTime()) /
								(1000 * 60 * 60 * 24),
						),
					})),
					secondary: weeks.map(week => ({
						date: week,
						label: format(week, 'MM/dd'),
						width: 7,
					})),
				}
			}
			case 'year': {
				const years: Array<{ date: Date; label: string; width: number }> = []
				let currentDate = new Date(timelineRange.minDate)
				while (currentDate <= timelineRange.maxDate) {
					const yearStart = new Date(currentDate.getFullYear(), 0, 1)
					const yearEnd = new Date(currentDate.getFullYear(), 11, 31)
					const actualStart =
						yearStart < timelineRange.minDate
							? timelineRange.minDate
							: yearStart
					const actualEnd =
						yearEnd > timelineRange.maxDate ? timelineRange.maxDate : yearEnd
					const width = Math.ceil(
						(actualEnd.getTime() - actualStart.getTime()) /
							(1000 * 60 * 60 * 24),
					)
					years.push({
						date: yearStart,
						label: format(yearStart, 'yyyy'),
						width,
					})
					currentDate = new Date(yearEnd)
					currentDate.setDate(currentDate.getDate() + 1)
				}
				const months = eachMonthOfInterval({
					start: timelineRange.minDate,
					end: timelineRange.maxDate,
				})
				return {
					primary: years,
					secondary: months.map(month => ({
						date: month,
						label: format(month, 'MMM'),
						width: Math.ceil(
							(endOfMonth(month).getTime() - startOfMonth(month).getTime()) /
								(1000 * 60 * 60 * 24),
						),
					})),
				}
			}
		}
	}, [timelineRange, zoomLevel])

	// Optimistic task updates state
	const [optimisticUpdates, setOptimisticUpdates] = useState<
		Map<string, { startDate: Date; endDate: Date }>
	>(new Map())

	// Update task dates with optimistic UI
	const updateTaskDates = useCallback(
		async (taskId: string, newStartDate: Date, newEndDate: Date) => {
			if (!canEdit) return

			const task = tasks.find(t => t.id === taskId)
			if (!task) return

			// Validate dates
			if (newStartDate > newEndDate) {
				toast({
					title: 'Invalid dates',
					description: 'Start date must be before end date',
					variant: 'destructive',
				})
				return
			}

			// Optimistically update UI immediately
			setOptimisticUpdates(prev => {
				const next = new Map(prev)
				next.set(taskId, { startDate: newStartDate, endDate: newEndDate })
				return next
			})

			try {
				// Find the data item
				const dataItem = task.dataItem

				// Convert property data to editor format
				const propertyData: PropertyDataForEditorFragment[] =
					dataItem.propertyData.map(pd => {
						if (pd.propertyId === startDatePropertyId) {
							return {
								propertyId: pd.propertyId,
								value: {
									__typename: 'DateValue',
									date: format(newStartDate, 'yyyy-MM-dd'),
								},
							}
						}
						if (pd.propertyId === endDatePropertyId) {
							return {
								propertyId: pd.propertyId,
								value: {
									__typename: 'DateValue',
									date: format(newEndDate, 'yyyy-MM-dd'),
								},
							}
						}
						// Convert other property data to editor format
						const value = pd.value as Record<string, unknown>
						if (value && 'date' in value) {
							return {
								propertyId: pd.propertyId,
								value: {
									__typename: 'DateValue',
									date: value.date as string,
								},
							}
						}
						return pd as PropertyDataForEditorFragment
					})

				// Ensure both date properties exist
				if (!propertyData.find(pd => pd.propertyId === startDatePropertyId)) {
					propertyData.push({
						propertyId: startDatePropertyId,
						value: {
							__typename: 'DateValue',
							date: format(newStartDate, 'yyyy-MM-dd'),
						},
					})
				}
				if (!propertyData.find(pd => pd.propertyId === endDatePropertyId)) {
					propertyData.push({
						propertyId: endDatePropertyId,
						value: {
							__typename: 'DateValue',
							date: format(newEndDate, 'yyyy-MM-dd'),
						},
					})
				}

				// Convert DataFieldOnRepoPageFragment to DataForDataDetailFragment
				const dataForDetail: DataForDataDetailFragment = {
					id: dataItem.id,
					name: dataItem.name,
					propertyData,
				}

				// Update data
				await updateData({
					org,
					repo,
					dataId: taskId,
					properties: editorProperties,
					input: dataForDetail,
				})

				// Clear optimistic update after successful save
				setOptimisticUpdates(prev => {
					const next = new Map(prev)
					next.delete(taskId)
					return next
				})

				toast({
					title: 'Task updated',
					description: 'Task dates have been updated successfully',
				})

				router.refresh()
			} catch (error) {
				// Revert optimistic update on error
				setOptimisticUpdates(prev => {
					const next = new Map(prev)
					next.delete(taskId)
					return next
				})

				console.error('Failed to update task dates:', error)
				toast({
					title: 'Update failed',
					description: error instanceof Error ? error.message : 'Unknown error',
					variant: 'destructive',
				})
			}
		},
		[
			canEdit,
			tasks,
			startDatePropertyId,
			endDatePropertyId,
			editorProperties,
			org,
			repo,
			toast,
			router,
		],
	)

	// Drag & Drop feature is temporarily disabled
	// // Handle drag move
	// const handleDragMove = useCallback(
	// 	(e: globalThis.MouseEvent) => {
	// 		if (!dragState.current || !timelineRange || !timelineRef.current) return
	//
	// 		const rect = timelineRef.current.getBoundingClientRect()
	// 		const currentX = e.clientX - rect.left
	// 		const deltaX = currentX - dragState.current.startX
	//
	// 		// Consider it a drag if moved more than 5 pixels
	// 		if (!dragState.current.hasMoved && Math.abs(deltaX) > 5) {
	// 			dragState.current.hasMoved = true
	// 			setIsDragging(true)
	// 		}
	//
	// 		if (!dragState.current.hasMoved) return
	//
	// 		const deltaDays = Math.round((deltaX / rect.width) * daysInRange)
	//
	// 		let newStartDate = new Date(dragState.current.originalStartDate)
	// 		let newEndDate = new Date(dragState.current.originalEndDate)
	//
	// 		switch (dragState.current.dragType) {
	// 			case 'move':
	// 				newStartDate = addDays(newStartDate, deltaDays)
	// 				newEndDate = addDays(newEndDate, deltaDays)
	// 				break
	// 			case 'resize-start':
	// 				newStartDate = addDays(newStartDate, deltaDays)
	// 				if (newStartDate > newEndDate) {
	// 					newStartDate = newEndDate
	// 				}
	// 				break
	// 			case 'resize-end':
	// 				newEndDate = addDays(newEndDate, deltaDays)
	// 				if (newEndDate < newStartDate) {
	// 					newEndDate = newStartDate
	// 				}
	// 				break
	// 		}
	//
	// 		// Update dragging task state for visual feedback
	// 		setDraggingTask({
	// 			taskId: dragState.current.taskId,
	// 			startDate: newStartDate,
	// 			endDate: newEndDate,
	// 		})
	// 	},
	// 	[timelineRange, daysInRange],
	// )
	//
	// // Handle drag end
	// const handleDragEnd = useCallback(() => {
	// 	if (!dragState.current) return
	//
	// 	const wasDragging = dragState.current.hasMoved
	//
	// 	if (wasDragging && draggingTask) {
	// 		updateTaskDates(
	// 			draggingTask.taskId,
	// 			draggingTask.startDate,
	// 			draggingTask.endDate,
	// 		)
	// 	}
	//
	// 	setDraggingTask(null)
	// 	setIsDragging(false)
	// 	dragState.current = null
	// 	document.removeEventListener('mousemove', handleDragMove)
	// 	document.removeEventListener('mouseup', handleDragEnd)
	// }, [draggingTask, updateTaskDates, handleDragMove])
	//
	// // Handle drag start
	// const handleDragStart = useCallback(
	// 	(
	// 		e: React.MouseEvent<HTMLDivElement>,
	// 		task: GanttTask,
	// 		dragType: 'move' | 'resize-start' | 'resize-end',
	// 	) => {
	// 		if (!canEdit) return
	//
	// 		e.preventDefault()
	// 		e.stopPropagation()
	//
	// 		const rect = timelineRef.current?.getBoundingClientRect()
	// 		if (!rect || !timelineRange) return
	//
	// 		const startX = e.clientX - rect.left
	// 		dragState.current = {
	// 			taskId: task.id,
	// 			startX,
	// 			originalStartDate: new Date(task.startDate),
	// 			originalEndDate: new Date(task.endDate),
	// 			dragType,
	// 			hasMoved: false,
	// 		}
	//
	// 		document.addEventListener('mousemove', handleDragMove)
	// 		document.addEventListener('mouseup', handleDragEnd)
	// 	},
	// 	[canEdit, timelineRange, handleDragMove, handleDragEnd],
	// )

	// Handle task bar click for inline editing
	const handleTaskBarClick = useCallback(
		(e: React.MouseEvent<HTMLElement>, task: GanttTask) => {
			if (!canEdit) return

			e.preventDefault()
			e.stopPropagation()

			setEditingTask(task)
			setEditStartDate(new Date(task.startDate))
			setEditEndDate(new Date(task.endDate))
		},
		[canEdit],
	)

	// Handle save edit
	const handleSaveEdit = useCallback(async () => {
		if (!editingTask || !editStartDate || !editEndDate) return

		if (editStartDate > editEndDate) {
			toast({
				title: 'Invalid dates',
				description: 'Start date must be before end date',
				variant: 'destructive',
			})
			return
		}

		await updateTaskDates(editingTask.id, editStartDate, editEndDate)
		setEditingTask(null)
		setEditStartDate(undefined)
		setEditEndDate(undefined)
	}, [editingTask, editStartDate, editEndDate, updateTaskDates, toast])

	// Handle zoom
	const handleZoomIn = useCallback(() => {
		const levels: ZoomLevel[] = ['year', 'month', 'week', 'day']
		const currentIndex = levels.indexOf(zoomLevel)
		if (currentIndex < levels.length - 1) {
			setZoomLevel(levels[currentIndex + 1])
		}
	}, [zoomLevel])

	const handleZoomOut = useCallback(() => {
		const levels: ZoomLevel[] = ['year', 'month', 'week', 'day']
		const currentIndex = levels.indexOf(zoomLevel)
		if (currentIndex > 0) {
			setZoomLevel(levels[currentIndex - 1])
		}
	}, [zoomLevel])

	if (dateProperties.length === 0) {
		return (
			<div className='flex flex-col items-center justify-center py-16 text-center'>
				<p className='text-muted-foreground'>
					No Date properties found. Add Date properties to use Gantt view.
				</p>
			</div>
		)
	}

	return (
		<div className='flex flex-col h-full'>
			{/* Property selection and zoom controls */}
			<div className='border-b bg-muted/30 p-4 space-y-4'>
				<div className='flex items-center justify-between'>
					<div className='flex items-center gap-4'>
						<div className='flex items-center gap-2'>
							<span className='text-sm font-medium'>Start Date:</span>
							<Select
								value={startDatePropertyId}
								onValueChange={setStartDatePropertyId}
							>
								<SelectTrigger className='w-48'>
									<SelectValue placeholder='Select start date property' />
								</SelectTrigger>
								<SelectContent>
									{dateProperties.map(prop => (
										<SelectItem key={prop.id} value={prop.id}>
											{prop.name}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>
						<div className='flex items-center gap-2'>
							<span className='text-sm font-medium'>End Date:</span>
							<Select
								value={endDatePropertyId}
								onValueChange={setEndDatePropertyId}
							>
								<SelectTrigger className='w-48'>
									<SelectValue placeholder='Select end date property' />
								</SelectTrigger>
								<SelectContent>
									{dateProperties.map(prop => (
										<SelectItem key={prop.id} value={prop.id}>
											{prop.name}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>
					</div>
					{canEdit && (
						<div className='flex items-center gap-2'>
							<Button
								variant='outline'
								size='sm'
								onClick={handleZoomOut}
								disabled={zoomLevel === 'year'}
							>
								<ZoomOut className='h-4 w-4 mr-2' />
								Zoom Out
							</Button>
							<Button
								variant='outline'
								size='sm'
								onClick={handleZoomIn}
								disabled={zoomLevel === 'day'}
							>
								<ZoomIn className='h-4 w-4 mr-2' />
								Zoom In
							</Button>
							<span className='text-sm text-muted-foreground capitalize'>
								{zoomLevel}
							</span>
						</div>
					)}
				</div>
				{tasks.length === 0 && (
					<p className='text-sm text-muted-foreground'>
						No tasks with valid start and end dates found.
					</p>
				)}
			</div>

			{/* Gantt chart area */}
			<div className='flex-1 overflow-auto' ref={timelineRef}>
				{tasks.length > 0 && timelineRange ? (
					<div className='min-w-full'>
						{/* Timeline header */}
						<div className='sticky top-0 bg-background z-20 border-b'>
							{/* Primary header row */}
							<div className='flex border-b'>
								<div className='w-64 flex-shrink-0 border-r bg-muted/30 px-3 py-2 text-xs font-medium text-muted-foreground'>
									Task
								</div>
								<div className='flex-1 flex'>
									{headers.primary.map((header, idx) => (
										<div
											key={idx}
											className='border-r bg-muted/20 px-2 py-2 text-xs font-medium text-foreground'
											style={{
												width: `${(header.width / daysInRange) * 100}%`,
											}}
										>
											{header.label}
										</div>
									))}
								</div>
							</div>
							{/* Secondary header row */}
							{headers.secondary.length > 0 && (
								<div className='flex border-b'>
									<div className='w-64 flex-shrink-0 border-r bg-muted/30 px-3 py-1 text-xs text-muted-foreground'>
										{zoomLevel === 'month' ? 'Week' : 'Month'}
									</div>
									<div className='flex-1 flex'>
										{headers.secondary.map((header, idx) => (
											<div
												key={idx}
												className='border-r bg-muted/10 px-1 py-1 text-xs text-muted-foreground'
												style={{
													width: `${(header.width / daysInRange) * 100}%`,
												}}
											>
												{header.label}
											</div>
										))}
									</div>
								</div>
							)}
						</div>

						{/* Task rows */}
						<div className='divide-y'>
							{tasks.map(task => {
								// Use optimistic update if available
								const optimisticUpdate = optimisticUpdates.get(task.id)
								const displayTask = optimisticUpdate
									? {
											...task,
											startDate: optimisticUpdate.startDate,
											endDate: optimisticUpdate.endDate,
										}
									: task

								const startOffset =
									(displayTask.startDate.getTime() -
										timelineRange.minDate.getTime()) /
									(1000 * 60 * 60 * 24)
								const duration =
									(displayTask.endDate.getTime() -
										displayTask.startDate.getTime()) /
									(1000 * 60 * 60 * 24)
								const widthPercent = (duration / daysInRange) * 100
								const leftPercent = (startOffset / daysInRange) * 100

								return (
									<div
										key={task.id}
										className='flex items-center min-h-[48px] hover:bg-muted/30 transition-colors'
									>
										{/* Task name column */}
										<div className='w-64 flex-shrink-0 border-r px-3 py-2'>
											<NextLink
												href={`/v1beta/${org}/${repo}/data/${task.id}`}
												className='font-medium text-primary hover:underline truncate block'
											>
												{task.name}
											</NextLink>
											<div className='text-xs text-muted-foreground mt-1'>
												{format(displayTask.startDate, 'MMM dd')} -{' '}
												{format(displayTask.endDate, 'MMM dd')}
											</div>
										</div>

										{/* Timeline column */}
										<div className='flex-1 relative h-full min-h-[48px] bg-muted/20 flex items-center'>
											{/* Grid lines */}
											<div className='absolute inset-0 flex'>
												{headers.secondary.length > 0
													? headers.secondary.map((header, idx) => (
															<div
																key={idx}
																className='border-r border-border/30'
																style={{
																	width: `${(header.width / daysInRange) * 100}%`,
																}}
															/>
														))
													: headers.primary.map((header, idx) => (
															<div
																key={idx}
																className='border-r border-border/30'
																style={{
																	width: `${(header.width / daysInRange) * 100}%`,
																}}
															/>
														))}
											</div>

											{/* Task bar */}
											{canEdit ? (
												<div className='relative w-full h-full'>
													{/* Task bar - Drag & Drop disabled, click to edit */}
													<button
														type='button'
														className='absolute top-1/2 -translate-y-1/2 h-8 bg-primary rounded-md shadow-sm border border-primary/20 flex items-center px-2 text-xs font-medium text-primary-foreground transition-all hover:shadow-md hover:scale-105 cursor-pointer z-10'
														style={{
															left: `${leftPercent}%`,
															width: `${widthPercent}%`,
															minWidth: '40px',
														}}
														title={`${displayTask.name}: ${format(displayTask.startDate, 'MMM dd, yyyy')} - ${format(displayTask.endDate, 'MMM dd, yyyy')} (${Math.ceil(duration)} days)`}
														onClick={e => {
															handleTaskBarClick(e, displayTask)
														}}
													>
														<span className='truncate'>
															{Math.ceil(duration)}d
														</span>
													</button>
												</div>
											) : (
												<div
													className='absolute top-1/2 -translate-y-1/2 h-8 bg-primary rounded-md shadow-sm border border-primary/20 flex items-center px-2 text-xs font-medium text-primary-foreground transition-all hover:shadow-md'
													style={{
														left: `${leftPercent}%`,
														width: `${widthPercent}%`,
														minWidth: '40px',
													}}
													title={`${displayTask.name}: ${format(displayTask.startDate, 'MMM dd, yyyy')} - ${format(displayTask.endDate, 'MMM dd, yyyy')} (${Math.ceil(duration)} days)`}
												>
													<span className='truncate'>
														{Math.ceil(duration)}d
													</span>
												</div>
											)}
										</div>
									</div>
								)
							})}
						</div>
					</div>
				) : (
					<div className='flex flex-col items-center justify-center py-16 text-center'>
						<p className='text-muted-foreground'>
							Select start and end date properties to view tasks in Gantt chart.
						</p>
					</div>
				)}
			</div>

			{/* Edit dialog */}
			<Dialog
				open={editingTask !== null}
				onOpenChange={open => {
					if (!open) {
						setEditingTask(null)
						setEditStartDate(undefined)
						setEditEndDate(undefined)
					}
				}}
			>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Edit Task Dates</DialogTitle>
					</DialogHeader>
					<div className='space-y-4'>
						<div className='flex items-center gap-4'>
							<span className='text-sm font-medium w-24 flex-shrink-0'>
								Start Date
							</span>
							<DatePicker
								date={editStartDate}
								onDateChange={setEditStartDate}
								placeholder='Pick a start date'
							/>
						</div>
						<div className='flex items-center gap-4'>
							<span className='text-sm font-medium w-24 flex-shrink-0'>
								End Date
							</span>
							<DatePicker
								date={editEndDate}
								onDateChange={setEditEndDate}
								placeholder='Pick an end date'
							/>
						</div>
						<div className='flex justify-end gap-2 pt-4'>
							<Button
								variant='outline'
								onClick={() => {
									setEditingTask(null)
									setEditStartDate(undefined)
									setEditEndDate(undefined)
								}}
							>
								Cancel
							</Button>
							<Button onClick={handleSaveEdit}>Save</Button>
						</div>
					</div>
				</DialogContent>
			</Dialog>
		</div>
	)
}
