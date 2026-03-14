'use client'

import {
	GoogleMap,
	Marker,
	InfoWindow,
	useJsApiLoader,
} from '@react-google-maps/api'
import { useState, useMemo, useCallback } from 'react'
import NextLink from 'next/link'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { ExternalLink, MapPin, X } from 'lucide-react'
import { useTranslation } from '@/lib/i18n/useTranslation'

const apiKey = process.env.NEXT_PUBLIC_GOOGLE_MAPS_API_KEY || ''

interface DataLocation {
	id: string
	name: string
	latitude: number
	longitude: number
}

interface DataLocationsMapProps {
	locations: DataLocation[]
	org: string
	repo: string
}

const containerStyle = {
	width: '100%',
	height: '350px',
	borderRadius: '8px',
}

// Calculate center and bounds from locations
function calculateMapCenter(locations: DataLocation[]) {
	if (locations.length === 0) {
		return { lat: 35.6895, lng: 139.6917 } // Tokyo default
	}

	const sumLat = locations.reduce((sum, loc) => sum + loc.latitude, 0)
	const sumLng = locations.reduce((sum, loc) => sum + loc.longitude, 0)

	return {
		lat: sumLat / locations.length,
		lng: sumLng / locations.length,
	}
}

// Fallback without API key
function DataLocationsMapFallback({
	locations,
	org,
	repo,
}: DataLocationsMapProps) {
	return (
		<div className='space-y-2'>
			<div className='flex h-[350px] items-center justify-center rounded-lg bg-muted text-sm text-muted-foreground border-2 border-dashed border-border'>
				<div className='text-center'>
					<div className='text-4xl mb-2'>🗺️</div>
					<div className='font-medium'>{locations.length} locations</div>
					{!apiKey && (
						<div className='text-xs mt-2 text-amber-600'>
							Set NEXT_PUBLIC_GOOGLE_MAPS_API_KEY for map view
						</div>
					)}
				</div>
			</div>
			<ul className='divide-y rounded-lg border'>
				{locations.map(loc => (
					<li key={loc.id} className='px-4 py-2'>
						<NextLink
							href={`/v1beta/${org}/${repo}/data/${loc.id}`}
							className='flex items-center justify-between hover:text-primary'
						>
							<span className='font-medium'>{loc.name}</span>
							<span className='text-xs text-muted-foreground'>
								📍 {loc.latitude.toFixed(4)}, {loc.longitude.toFixed(4)}
							</span>
						</NextLink>
					</li>
				))}
			</ul>
		</div>
	)
}

export function DataLocationsMap({
	locations,
	org,
	repo,
}: DataLocationsMapProps) {
	if (!apiKey) {
		return (
			<DataLocationsMapFallback locations={locations} org={org} repo={repo} />
		)
	}

	return <DataLocationsMapWithApi locations={locations} org={org} repo={repo} />
}

function DataLocationsMapWithApi({
	locations,
	org,
	repo,
}: DataLocationsMapProps) {
	const { locale } = useTranslation()

	const { isLoaded, loadError } = useJsApiLoader({
		googleMapsApiKey: apiKey,
		language: locale,
	})

	const [map, setMap] = useState<google.maps.Map | null>(null)
	const [selectedLocation, setSelectedLocation] = useState<DataLocation | null>(
		null,
	)
	const [hoveredLocation, setHoveredLocation] = useState<DataLocation | null>(
		null,
	)

	const center = useMemo(() => calculateMapCenter(locations), [locations])

	// Fit bounds to show all markers when map loads or locations change
	const onMapLoad = useCallback(
		(mapInstance: google.maps.Map) => {
			setMap(mapInstance)
			if (locations.length > 1) {
				const bounds = new google.maps.LatLngBounds()
				for (const loc of locations) {
					bounds.extend({ lat: loc.latitude, lng: loc.longitude })
				}
				mapInstance.fitBounds(bounds, {
					top: 50,
					right: 50,
					bottom: 50,
					left: 50,
				})
			}
		},
		[locations],
	)

	const handleMarkerClick = useCallback(
		(location: DataLocation) => {
			setSelectedLocation(location)
			setHoveredLocation(null) // Clear hover when clicked
			// Center and zoom to selected marker
			if (map) {
				map.panTo({ lat: location.latitude, lng: location.longitude })
				map.setZoom(13)
			}
		},
		[map],
	)

	const handleMarkerMouseOver = useCallback(
		(location: DataLocation) => {
			// Only show hover preview if not already selected
			if (selectedLocation?.id !== location.id) {
				setHoveredLocation(location)
			}
		},
		[selectedLocation],
	)

	const handleMarkerMouseOut = useCallback(() => {
		setHoveredLocation(null)
	}, [])

	const handleClearSelection = useCallback(() => {
		setSelectedLocation(null)
	}, [])

	if (loadError) {
		return (
			<DataLocationsMapFallback locations={locations} org={org} repo={repo} />
		)
	}

	if (!isLoaded) {
		return (
			<div className='flex h-[350px] items-center justify-center rounded-lg bg-muted text-sm text-muted-foreground'>
				Loading map...
			</div>
		)
	}

	return (
		<div className='space-y-3'>
			<GoogleMap
				mapContainerStyle={containerStyle}
				center={center}
				zoom={locations.length === 1 ? 13 : 5}
				onLoad={onMapLoad}
				options={{
					disableDefaultUI: true,
					zoomControl: true,
					fullscreenControl: true,
				}}
			>
				{locations.map(location => (
					<Marker
						key={location.id}
						position={{ lat: location.latitude, lng: location.longitude }}
						title={location.name}
						onClick={() => handleMarkerClick(location)}
						onMouseOver={() => handleMarkerMouseOver(location)}
						onMouseOut={handleMarkerMouseOut}
						icon={
							selectedLocation?.id === location.id
								? {
										url: 'https://maps.google.com/mapfiles/ms/icons/blue-dot.png',
									}
								: undefined
						}
					/>
				))}

				{/* Hover preview InfoWindow */}
				{hoveredLocation && (
					<InfoWindow
						position={{
							lat: hoveredLocation.latitude,
							lng: hoveredLocation.longitude,
						}}
						options={{
							pixelOffset: new google.maps.Size(0, -30),
							disableAutoPan: true,
						}}
					>
						<div className='px-1 py-0.5'>
							<span className='font-medium text-sm text-gray-900'>
								{hoveredLocation.name}
							</span>
						</div>
					</InfoWindow>
				)}
			</GoogleMap>

			{/* Selected location card */}
			{selectedLocation ? (
				<Card className='border-primary/50 bg-primary/5'>
					<CardContent className='p-4'>
						<div className='flex items-start justify-between gap-3'>
							<div className='flex-1 min-w-0'>
								<div className='flex items-center gap-2 mb-1'>
									<MapPin className='h-4 w-4 text-primary shrink-0' />
									<h4 className='font-semibold text-base truncate'>
										{selectedLocation.name}
									</h4>
								</div>
								<p className='text-sm text-muted-foreground'>
									{selectedLocation.latitude.toFixed(6)},{' '}
									{selectedLocation.longitude.toFixed(6)}
								</p>
							</div>
							<div className='flex items-center gap-2 shrink-0'>
								<Button asChild size='sm' variant='default'>
									<NextLink
										href={`/v1beta/${org}/${repo}/data/${selectedLocation.id}`}
									>
										<ExternalLink className='h-4 w-4 mr-1' />
										View
									</NextLink>
								</Button>
								<Button
									size='sm'
									variant='ghost'
									onClick={handleClearSelection}
									aria-label='Clear selection'
								>
									<X className='h-4 w-4' />
								</Button>
							</div>
						</div>
					</CardContent>
				</Card>
			) : (
				<div className='text-xs text-muted-foreground text-center py-2'>
					{locations.length} location{locations.length !== 1 ? 's' : ''} plotted
					• Hover for preview, click for details
				</div>
			)}
		</div>
	)
}
