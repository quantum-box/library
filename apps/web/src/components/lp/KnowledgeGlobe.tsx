import type { LpLanguage } from '@/app/lp'
import { useEffect, useRef } from 'react'

type ThreeModule = typeof import('three')

const copy = {
	en: {
		caption: 'fig. 1 — Knowledge network on an earth scale',
		hint: 'Drag to rotate',
	},
	ja: {
		caption: '図1 — 地球規模のナレッジネットワーク',
		hint: 'ドラッグで回転',
	},
} satisfies Record<LpLanguage, { caption: string; hint: string }>

const COLORS = {
	graticule: 0xcbd5e1, // slate-300
	dots: 0x64748b, // slate-500
	nodes: 0x1d4ed8, // blue-700
	arcs: 0x2563eb, // blue-600
	pulses: 0x1d4ed8, // blue-700
	occluder: 0xffffff,
} as const

export function KnowledgeGlobe({ lang }: { lang: LpLanguage }) {
	const t = copy[lang]
	const containerRef = useRef<HTMLDivElement>(null)

	useEffect(() => {
		const container = containerRef.current
		if (!container) return
		let disposed = false
		let cleanup: (() => void) | undefined
		import('three').then(three => {
			if (disposed) return
			cleanup = createGlobe(three, container)
		})
		return () => {
			disposed = true
			cleanup?.()
		}
	}, [])

	return (
		<figure className='min-w-0 lg:mt-2'>
			<div className='overflow-hidden rounded-lg border border-slate-200 bg-white'>
				<div
					ref={containerRef}
					aria-hidden='true'
					className='aspect-square w-full cursor-grab touch-none active:cursor-grabbing'
				/>
				<figcaption className='flex items-baseline justify-between gap-4 border-t border-slate-200 bg-slate-50 px-5 py-3'>
					<span className='font-mono text-[11px] text-slate-500'>
						{t.caption}
					</span>
					<span className='hidden font-mono text-[11px] text-slate-400 sm:inline'>
						{t.hint}
					</span>
				</figcaption>
			</div>
		</figure>
	)
}

function fibonacciSphere(three: ThreeModule, count: number) {
	const points: InstanceType<ThreeModule['Vector3']>[] = []
	const golden = Math.PI * (3 - Math.sqrt(5))
	for (let i = 0; i < count; i++) {
		const y = 1 - (i / (count - 1)) * 2
		const radius = Math.sqrt(1 - y * y)
		const theta = golden * i
		points.push(
			new three.Vector3(
				Math.cos(theta) * radius,
				y,
				Math.sin(theta) * radius,
			),
		)
	}
	return points
}

function circlePoints(
	three: ThreeModule,
	radius: number,
	y: number,
	segments = 128,
) {
	const points: InstanceType<ThreeModule['Vector3']>[] = []
	for (let i = 0; i <= segments; i++) {
		const angle = (i / segments) * Math.PI * 2
		points.push(
			new three.Vector3(Math.cos(angle) * radius, y, Math.sin(angle) * radius),
		)
	}
	return points
}

function createGlobe(three: ThreeModule, container: HTMLElement) {
	const prefersReducedMotion = window.matchMedia(
		'(prefers-reduced-motion: reduce)',
	).matches

	const scene = new three.Scene()
	const camera = new three.PerspectiveCamera(38, 1, 0.1, 100)
	camera.position.set(0, 0.35, 3.7)
	camera.lookAt(0, 0, 0)

	const renderer = new three.WebGLRenderer({ antialias: true, alpha: true })
	renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
	// Size the drawing buffer manually and let CSS drive the layout width, so
	// the canvas never imposes a min-content width on the hero grid column.
	renderer.domElement.style.width = '100%'
	renderer.domElement.style.height = '100%'
	renderer.domElement.style.display = 'block'
	container.appendChild(renderer.domElement)

	const dotTexture = createDotTexture(three)

	// pitch (drag up/down) → tilt (fixed axial tilt) → spin (auto + drag left/right)
	const pitch = new three.Group()
	const tilt = new three.Group()
	const spin = new three.Group()
	tilt.rotation.z = -0.32
	pitch.add(tilt)
	tilt.add(spin)
	scene.add(pitch)

	// Opaque sphere hides the far side so the line work reads as a solid globe.
	const occluder = new three.Mesh(
		new three.SphereGeometry(0.992, 48, 32),
		new three.MeshBasicMaterial({ color: COLORS.occluder }),
	)
	spin.add(occluder)

	const graticuleMaterial = new three.LineBasicMaterial({
		color: COLORS.graticule,
		transparent: true,
		opacity: 0.8,
	})
	for (let lat = -60; lat <= 60; lat += 30) {
		const phi = (lat * Math.PI) / 180
		const ring = new three.Line(
			new three.BufferGeometry().setFromPoints(
				circlePoints(three, Math.cos(phi), Math.sin(phi)),
			),
			graticuleMaterial,
		)
		spin.add(ring)
	}
	for (let lon = 0; lon < 180; lon += 30) {
		const meridian = new three.Line(
			new three.BufferGeometry().setFromPoints(circlePoints(three, 1, 0)),
			graticuleMaterial,
		)
		meridian.rotation.z = Math.PI / 2
		meridian.rotation.y = (lon * Math.PI) / 180
		spin.add(meridian)
	}

	const surface = fibonacciSphere(three, 700)
	const dots = new three.Points(
		new three.BufferGeometry().setFromPoints(surface),
		new three.PointsMaterial({
			color: COLORS.dots,
			size: 0.018,
			map: dotTexture,
			alphaTest: 0.5,
			transparent: true,
			opacity: 0.9,
		}),
	)
	spin.add(dots)

	const nodePoints = surface.filter((_, index) => index % 50 === 25)
	const nodes = new three.Points(
		new three.BufferGeometry().setFromPoints(nodePoints),
		new three.PointsMaterial({
			color: COLORS.nodes,
			size: 0.045,
			map: dotTexture,
			alphaTest: 0.5,
			transparent: true,
		}),
	)
	spin.add(nodes)

	// Great-circle arcs between knowledge nodes. Points follow the surface
	// direction with a sine lift so the whole arc stays above the occluder.
	type Arc = {
		from: InstanceType<ThreeModule['Vector3']>
		to: InstanceType<ThreeModule['Vector3']>
		lift: number
	}
	const arcs: Arc[] = []
	const arcPoint = (
		arc: Arc,
		t: number,
		target: InstanceType<ThreeModule['Vector3']>,
	) =>
		target
			.copy(arc.from)
			.lerp(arc.to, t)
			.normalize()
			.multiplyScalar(1 + arc.lift * Math.sin(Math.PI * t))
	const arcMaterial = new three.LineBasicMaterial({
		color: COLORS.arcs,
		transparent: true,
		opacity: 0.7,
	})
	for (const step of [3, 5]) {
		for (let i = 0; i < nodePoints.length; i += 2) {
			const from = nodePoints[i]
			const to = nodePoints[(i + step) % nodePoints.length]
			const angle = from.angleTo(to)
			if (angle < 0.5 || angle > 1.9) continue
			const arc: Arc = { from, to, lift: 0.04 + 0.12 * (angle / Math.PI) }
			arcs.push(arc)
			const segments = 48
			const points = []
			for (let s = 0; s <= segments; s++) {
				points.push(arcPoint(arc, s / segments, new three.Vector3()))
			}
			spin.add(
				new three.Line(
					new three.BufferGeometry().setFromPoints(points),
					arcMaterial,
				),
			)
		}
	}

	// One pulse per arc: a dot travelling along the curve (knowledge in transit).
	const pulseGeometry = new three.BufferGeometry().setFromPoints(
		arcs.map(arc => arcPoint(arc, 0, new three.Vector3())),
	)
	const pulses = new three.Points(
		pulseGeometry,
		new three.PointsMaterial({
			color: COLORS.pulses,
			size: 0.04,
			map: dotTexture,
			alphaTest: 0.5,
			transparent: true,
		}),
	)
	if (!prefersReducedMotion) spin.add(pulses)

	let dragging = false
	let lastX = 0
	let lastY = 0
	const onPointerDown = (event: PointerEvent) => {
		dragging = true
		lastX = event.clientX
		lastY = event.clientY
		renderer.domElement.setPointerCapture(event.pointerId)
	}
	const onPointerMove = (event: PointerEvent) => {
		if (!dragging) return
		spin.rotation.y += (event.clientX - lastX) * 0.005
		pitch.rotation.x = Math.min(
			0.6,
			Math.max(-0.6, pitch.rotation.x + (event.clientY - lastY) * 0.003),
		)
		lastX = event.clientX
		lastY = event.clientY
	}
	const onPointerUp = () => {
		dragging = false
	}
	renderer.domElement.addEventListener('pointerdown', onPointerDown)
	renderer.domElement.addEventListener('pointermove', onPointerMove)
	renderer.domElement.addEventListener('pointerup', onPointerUp)
	renderer.domElement.addEventListener('pointercancel', onPointerUp)

	const resize = () => {
		const size = Math.max(1, container.clientWidth)
		renderer.setSize(size, size, false)
	}
	resize()
	const observer = new ResizeObserver(resize)
	observer.observe(container)

	let frame = 0
	let time = 0
	const positionAttribute = pulseGeometry.getAttribute('position')
	const pulsePosition = new three.Vector3()
	const animate = () => {
		frame = requestAnimationFrame(animate)
		if (!dragging && !prefersReducedMotion) {
			spin.rotation.y += 0.0016
		}
		if (!prefersReducedMotion) {
			time += 0.0022
			for (let i = 0; i < arcs.length; i++) {
				const point = arcPoint(arcs[i], (time + i / arcs.length) % 1, pulsePosition)
				positionAttribute.setXYZ(i, point.x, point.y, point.z)
			}
			positionAttribute.needsUpdate = true
		}
		renderer.render(scene, camera)
	}
	animate()

	return () => {
		cancelAnimationFrame(frame)
		observer.disconnect()
		renderer.domElement.removeEventListener('pointerdown', onPointerDown)
		renderer.domElement.removeEventListener('pointermove', onPointerMove)
		renderer.domElement.removeEventListener('pointerup', onPointerUp)
		renderer.domElement.removeEventListener('pointercancel', onPointerUp)
		scene.traverse(object => {
			if (object instanceof three.Mesh || object instanceof three.Line || object instanceof three.Points) {
				object.geometry.dispose()
				const material = object.material
				if (Array.isArray(material)) {
					for (const m of material) m.dispose()
				} else {
					material.dispose()
				}
			}
		})
		dotTexture.dispose()
		renderer.dispose()
		renderer.domElement.remove()
	}
}

function createDotTexture(three: ThreeModule) {
	const size = 64
	const canvas = document.createElement('canvas')
	canvas.width = size
	canvas.height = size
	const context = canvas.getContext('2d')
	if (context) {
		context.fillStyle = '#ffffff'
		context.beginPath()
		context.arc(size / 2, size / 2, size / 2 - 2, 0, Math.PI * 2)
		context.fill()
	}
	const texture = new three.CanvasTexture(canvas)
	texture.colorSpace = three.SRGBColorSpace
	return texture
}
