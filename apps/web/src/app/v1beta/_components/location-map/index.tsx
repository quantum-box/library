'use client'

import {
	GoogleMap,
	Marker,
	useJsApiLoader,
	Autocomplete,
} from '@react-google-maps/api'
import { useCallback, useEffect, useState, useRef } from 'react'
import {
	HoverCard,
	HoverCardContent,
	HoverCardTrigger,
} from '@/components/ui/hover-card'
import { Input } from '@/components/ui/input'
import { Search } from 'lucide-react'
import { useTranslation } from '@/lib/i18n/useTranslation'

const containerStyle = {
	width: '100%',
	height: '150px',
	borderRadius: '8px',
}

const defaultCenter = {
	lat: 35.6895,
	lng: 139.6917,
}

const apiKey = process.env.NEXT_PUBLIC_GOOGLE_MAPS_API_KEY || ''

// Libraries needed for Google Maps
const libraries: 'places'[] = ['places']

interface LocationMapProps {
	latitude: number
	longitude: number
	editable?: boolean
	onChange?: (lat: number, lng: number) => void
}

// Hook to get place name from coordinates (using Places API for POI, fallback to Geocoding)
export function useReverseGeocode(latitude: number, longitude: number) {
	const [address, setAddress] = useState<string | null>(null)
	const [loading, setLoading] = useState(true)
	const { locale } = useTranslation()

	useEffect(() => {
		if (!apiKey || !latitude || !longitude) {
			setLoading(false)
			return
		}

		// Try to find nearby POI first using Places API
		const service = new google.maps.places.PlacesService(
			document.createElement('div'),
		)

		// First try to find transit stations (train/subway/bus stations)
		service.nearbySearch(
			{
				location: { lat: latitude, lng: longitude },
				radius: 100, // 100 meters
				type: 'transit_station',
			},
			(transitResults, transitStatus) => {
				// If found transit station, use it
				if (
					transitStatus === google.maps.places.PlacesServiceStatus.OK &&
					transitResults &&
					transitResults.length > 0 &&
					transitResults[0].name
				) {
					setAddress(transitResults[0].name)
					setLoading(false)
					return
				}

				// Otherwise, search for any POI nearby
				service.nearbySearch(
					{
						location: { lat: latitude, lng: longitude },
						radius: 50, // 50 meters
					},
					(results, status) => {
						if (
							status === google.maps.places.PlacesServiceStatus.OK &&
							results &&
							results.length > 0 &&
							results[0].name
						) {
							setAddress(results[0].name)
							setLoading(false)
							return
						}

						// Fallback to reverse geocoding
						const geocoder = new google.maps.Geocoder()
						geocoder.geocode(
							{ location: { lat: latitude, lng: longitude } },
							(geoResults, geoStatus) => {
								if (geoStatus === 'OK' && geoResults && geoResults[0]) {
									// Check for POI in geocode results
									const poiResult = geoResults.find(
										r =>
											r.types.includes('point_of_interest') ||
											r.types.includes('establishment') ||
											r.types.includes('transit_station'),
									)

									if (poiResult) {
										// Extract POI name from formatted address
										const name = poiResult.formatted_address.split(',')[0]
										setAddress(name)
									} else {
										// Get a shorter address (locality, country)
										const addressComponents = geoResults[0].address_components
										const locality = addressComponents?.find(c =>
											c.types.includes('locality'),
										)?.long_name
										const adminArea = addressComponents?.find(c =>
											c.types.includes('administrative_area_level_1'),
										)?.short_name
										const country = addressComponents?.find(c =>
											c.types.includes('country'),
										)?.short_name

										if (locality && (adminArea || country)) {
											setAddress(`${locality}, ${adminArea || country}`)
										} else {
											const formatted = geoResults[0].formatted_address
											const parts = formatted.split(', ')
											setAddress(parts.slice(0, 2).join(', '))
										}
									}
								}
								setLoading(false)
							},
						)
					},
				)
			},
		)
	}, [latitude, longitude])

	return { address, loading }
}

// Fallback display when API key is not available
function LocationFallback({
	latitude,
	longitude,
	editable,
	onChange,
}: LocationMapProps) {
	const [lat, setLat] = useState(latitude ?? 0)
	const [lng, setLng] = useState(longitude ?? 0)

	return (
		<div className='space-y-2'>
			<div className='flex h-[150px] items-center justify-center rounded-lg bg-muted text-sm text-muted-foreground border-2 border-dashed border-border'>
				<div className='text-center'>
					<div className='text-2xl mb-1'>📍</div>
					<div>
						{(lat ?? 0).toFixed(6)}, {(lng ?? 0).toFixed(6)}
					</div>
					{!apiKey && (
						<div className='text-xs mt-2 text-amber-600'>
							Set NEXT_PUBLIC_GOOGLE_MAPS_API_KEY for map view
						</div>
					)}
				</div>
			</div>
			{editable && (
				<div className='flex gap-2'>
					<input
						type='number'
						value={lat}
						onChange={e => {
							const v = Number.parseFloat(e.target.value) || 0
							setLat(v)
							onChange?.(v, lng)
						}}
						className='flex-1 h-8 px-2 rounded border text-sm'
						placeholder='Latitude'
						step='any'
					/>
					<input
						type='number'
						value={lng}
						onChange={e => {
							const v = Number.parseFloat(e.target.value) || 0
							setLng(v)
							onChange?.(lat, v)
						}}
						className='flex-1 h-8 px-2 rounded border text-sm'
						placeholder='Longitude'
						step='any'
					/>
				</div>
			)}
		</div>
	)
}

export function LocationMap({
	latitude,
	longitude,
	editable = false,
	onChange,
}: LocationMapProps) {
	// Use default values if latitude/longitude are undefined
	const lat = latitude ?? defaultCenter.lat
	const lng = longitude ?? defaultCenter.lng

	// If no API key, show fallback immediately
	if (!apiKey) {
		return (
			<LocationFallback
				latitude={lat}
				longitude={lng}
				editable={editable}
				onChange={onChange}
			/>
		)
	}

	return (
		<LocationMapWithApi
			latitude={lat}
			longitude={lng}
			editable={editable}
			onChange={onChange}
		/>
	)
}

// Internal component that uses Google Maps API
function LocationMapWithApi({
	latitude,
	longitude,
	editable = false,
	onChange,
}: LocationMapProps) {
	// Use default values if latitude/longitude are undefined
	const lat = latitude ?? defaultCenter.lat
	const lng = longitude ?? defaultCenter.lng
	const { locale } = useTranslation()

	const { isLoaded, loadError } = useJsApiLoader({
		googleMapsApiKey: apiKey,
		libraries,
		language: locale,
	})

	const [position, setPosition] = useState({
		lat: lat,
		lng: lng,
	})

	const autocompleteRef = useRef<google.maps.places.Autocomplete | null>(null)
	const inputRef = useRef<HTMLInputElement | null>(null)

	const handleClick = useCallback(
		(e: google.maps.MapMouseEvent) => {
			if (!editable || !e.latLng) return

			const newLat = e.latLng.lat()
			const newLng = e.latLng.lng()
			setPosition({ lat: newLat, lng: newLng })
			onChange?.(newLat, newLng)
		},
		[editable, onChange],
	)

	const onAutocompleteLoad = useCallback(
		(autocomplete: google.maps.places.Autocomplete) => {
			autocompleteRef.current = autocomplete
			// Request geometry field to get coordinates
			autocomplete.setFields(['geometry', 'name', 'formatted_address'])
		},
		[],
	)

	const onPlaceChanged = useCallback(() => {
		if (autocompleteRef.current) {
			const place = autocompleteRef.current.getPlace()
			if (place.geometry?.location) {
				const newLat = place.geometry.location.lat()
				const newLng = place.geometry.location.lng()
				setPosition({ lat: newLat, lng: newLng })
				onChange?.(newLat, newLng)
			} else {
				// Fallback: use Geocoding API with input text
				const inputText = inputRef.current?.value || ''
				if (inputText) {
					const geocoder = new google.maps.Geocoder()
					geocoder.geocode({ address: inputText }, (results, status) => {
						if (status === 'OK' && results && results[0]?.geometry?.location) {
							const newLat = results[0].geometry.location.lat()
							const newLng = results[0].geometry.location.lng()
							setPosition({ lat: newLat, lng: newLng })
							onChange?.(newLat, newLng)
						}
					})
				}
			}
		}
	}, [onChange])

	if (loadError) {
		return (
			<LocationFallback
				latitude={lat}
				longitude={lng}
				editable={editable}
				onChange={onChange}
			/>
		)
	}

	if (!isLoaded) {
		return (
			<div className='flex h-[150px] items-center justify-center rounded-lg bg-muted text-sm text-muted-foreground'>
				Loading map...
			</div>
		)
	}

	return (
		<div className='space-y-2'>
			{editable && (
				<div className='relative'>
					<Autocomplete
						onLoad={onAutocompleteLoad}
						onPlaceChanged={onPlaceChanged}
						options={{ fields: ['geometry', 'name', 'formatted_address'] }}
					>
						<div className='relative'>
							<Search className='absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground' />
							<Input
								ref={inputRef}
								type='text'
								placeholder='Search for a place...'
								className='pl-9 h-9'
							/>
						</div>
					</Autocomplete>
				</div>
			)}
			<GoogleMap
				mapContainerStyle={containerStyle}
				center={position}
				zoom={13}
				onClick={handleClick}
				options={{
					disableDefaultUI: !editable,
					zoomControl: editable,
					streetViewControl: false,
					mapTypeControl: false,
					fullscreenControl: false,
				}}
			>
				<Marker position={position} />
			</GoogleMap>
		</div>
	)
}

// Compact version for table cells - shows address on hover shows map
export function LocationMapCompact({
	latitude,
	longitude,
}: {
	latitude: number
	longitude: number
}) {
	// If latitude or longitude is undefined/null, show placeholder
	if (
		latitude === undefined ||
		latitude === null ||
		longitude === undefined ||
		longitude === null
	) {
		return <span className='text-xs text-muted-foreground'>📍 No location</span>
	}

	// If no API key, show text with HoverCard (no map)
	if (!apiKey) {
		return (
			<HoverCard openDelay={200} closeDelay={100}>
				<HoverCardTrigger asChild>
					<button
						type='button'
						className='inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer'
					>
						<span>📍</span>
						<span className='max-w-[150px] truncate'>
							{latitude.toFixed(4)}, {longitude.toFixed(4)}
						</span>
					</button>
				</HoverCardTrigger>
				<HoverCardContent className='w-64 p-3' side='top' align='start'>
					<div className='space-y-2'>
						<div className='flex h-24 items-center justify-center rounded-lg bg-muted text-sm text-muted-foreground border-2 border-dashed border-border'>
							<div className='text-center'>
								<div className='text-2xl mb-1'>🗺️</div>
								<div className='text-xs'>Map unavailable</div>
							</div>
						</div>
						<div className='text-xs text-muted-foreground'>
							<div className='font-medium text-foreground'>Coordinates</div>
							<div>
								{latitude.toFixed(6)}, {longitude.toFixed(6)}
							</div>
						</div>
					</div>
				</HoverCardContent>
			</HoverCard>
		)
	}

	return <LocationMapCompactWithApi latitude={latitude} longitude={longitude} />
}

// Internal component that uses Google Maps API with HoverCard
function LocationMapCompactWithApi({
	latitude,
	longitude,
}: {
	latitude: number
	longitude: number
}) {
	const { locale } = useTranslation()

	const { isLoaded, loadError } = useJsApiLoader({
		googleMapsApiKey: apiKey,
		libraries,
		language: locale,
	})

	const { address, loading: addressLoading } = useReverseGeocode(
		isLoaded ? latitude : 0,
		isLoaded ? longitude : 0,
	)

	// Show loading or fallback text
	if (loadError || !isLoaded) {
		return (
			<span className='text-xs text-muted-foreground'>
				📍 {latitude.toFixed(4)}, {longitude.toFixed(4)}
			</span>
		)
	}

	const displayText = addressLoading
		? 'Loading...'
		: address || `${latitude.toFixed(4)}, ${longitude.toFixed(4)}`

	return (
		<HoverCard openDelay={200} closeDelay={100}>
			<HoverCardTrigger asChild>
				<button
					type='button'
					className='inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer'
				>
					<span>📍</span>
					<span className='max-w-[150px] truncate'>{displayText}</span>
				</button>
			</HoverCardTrigger>
			<HoverCardContent className='w-80 p-2' side='top' align='start'>
				<div className='space-y-2'>
					<div className='h-32 w-full rounded overflow-hidden'>
						<GoogleMap
							mapContainerStyle={{ width: '100%', height: '128px' }}
							center={{ lat: latitude, lng: longitude }}
							zoom={13}
							options={{
								disableDefaultUI: true,
								draggable: false,
								scrollwheel: false,
							}}
						>
							<Marker position={{ lat: latitude, lng: longitude }} />
						</GoogleMap>
					</div>
					<div className='text-xs text-muted-foreground'>
						{address && (
							<div className='font-medium text-foreground'>{address}</div>
						)}
						<div>
							{latitude.toFixed(6)}, {longitude.toFixed(6)}
						</div>
					</div>
				</div>
			</HoverCardContent>
		</HoverCard>
	)
}
