'use client'

import type { Meta, StoryFn } from '@storybook/react'
import { fn } from '@storybook/test'
import { LocationMap, LocationMapCompact } from './index'

const meta = {
	title: 'V1Beta/LocationMap',
	component: LocationMap,
	parameters: {
		layout: 'padded',
	},
	tags: ['location-map'],
} satisfies Meta<typeof LocationMap>

export default meta

// Tokyo Station
export const Default: StoryFn<typeof LocationMap> = () => (
	<div className='w-[400px]'>
		<LocationMap latitude={35.6812} longitude={139.7671} />
	</div>
)

// Editable map
export const Editable: StoryFn<typeof LocationMap> = () => {
	const handleChange = fn()
	return (
		<div className='w-[400px]'>
			<p className='mb-2 text-sm text-muted-foreground'>
				Click on the map to change the marker position
			</p>
			<LocationMap
				latitude={35.6812}
				longitude={139.7671}
				editable
				onChange={handleChange}
			/>
		</div>
	)
}

// New York
export const NewYork: StoryFn<typeof LocationMap> = () => (
	<div className='w-[400px]'>
		<LocationMap latitude={40.7128} longitude={-74.006} />
	</div>
)

// Sydney Opera House
export const Sydney: StoryFn<typeof LocationMap> = () => (
	<div className='w-[400px]'>
		<LocationMap latitude={-33.8568} longitude={151.2153} />
	</div>
)

// Compact version for tables
export const Compact: StoryFn<typeof LocationMapCompact> = () => (
	<div className='space-y-4'>
		<div>
			<p className='mb-1 text-xs text-muted-foreground'>Tokyo</p>
			<LocationMapCompact latitude={35.6812} longitude={139.7671} />
		</div>
		<div>
			<p className='mb-1 text-xs text-muted-foreground'>New York</p>
			<LocationMapCompact latitude={40.7128} longitude={-74.006} />
		</div>
		<div>
			<p className='mb-1 text-xs text-muted-foreground'>London</p>
			<LocationMapCompact latitude={51.5074} longitude={-0.1278} />
		</div>
	</div>
)
