'use client'

import { useToast } from '@/components/ui/use-toast'
import { useEffect } from 'react'

export const ToastClient = ({
	title,
	description,
	variant,
}: {
	title: string
	description: string
	variant: 'default' | 'destructive' | null | undefined
}) => {
	const { toast } = useToast()

	const handleClick = async () => {
		if (typeof window === 'undefined') return
		await new Promise(resolve => setTimeout(resolve, 1))
		toast({
			variant,
			title,
			description,
		})
	}

	useEffect(() => {
		handleClick()
	}, [])

	return <></>
}
