export function PlanetMark({ className }: { className?: string }) {
	return (
		<svg
			xmlns='http://www.w3.org/2000/svg'
			viewBox='0 0 24 24'
			fill='none'
			className={className}
		>
			<ellipse
				cx='12'
				cy='12'
				rx='10'
				ry='4.2'
				transform='rotate(-18 12 12)'
				stroke='currentColor'
				strokeWidth='1.5'
			/>
			<circle
				cx='12'
				cy='12'
				r='5.2'
				stroke='currentColor'
				strokeWidth='1.8'
			/>
			<circle
				cx='21.5'
				cy='8.9'
				r='1.8'
				fill='currentColor'
				className='text-blue-400'
			/>
		</svg>
	)
}
