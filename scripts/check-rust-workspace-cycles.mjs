#!/usr/bin/env node

import { execFileSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(
	path.dirname(fileURLToPath(import.meta.url)),
	'..',
)

function loadCargoMetadata() {
	try {
		const stdout = execFileSync(
			'cargo',
			['metadata', '--format-version=1', '--no-deps'],
			{
				cwd: repoRoot,
				encoding: 'utf8',
				stdio: ['ignore', 'pipe', 'pipe'],
			},
		)
		return JSON.parse(stdout)
	} catch (error) {
		const stderr = error?.stderr?.toString?.() ?? ''
		console.error('Failed to run `cargo metadata --format-version=1 --no-deps`.')
		if (stderr.trim()) {
			console.error(stderr.trim())
		}
		process.exit(1)
	}
}

function packageLabel(pkg) {
	return `${pkg.name} (${path.relative(repoRoot, path.dirname(pkg.manifest_path))})`
}

function buildWorkspaceGraph(metadata) {
	const workspaceMemberIds = new Set(metadata.workspace_members)
	const workspacePackages = metadata.packages.filter((pkg) =>
		workspaceMemberIds.has(pkg.id),
	)

	const packageByManifestDir = new Map()
	for (const pkg of workspacePackages) {
		packageByManifestDir.set(path.dirname(pkg.manifest_path), pkg)
	}

	const graph = new Map()
	for (const pkg of workspacePackages) {
		const dependencies = new Set()

		for (const dependency of pkg.dependencies) {
			if (!dependency.path) {
				continue
			}

			const dependencyPackage = packageByManifestDir.get(
				path.resolve(dependency.path),
			)
			if (dependencyPackage && dependencyPackage.id !== pkg.id) {
				dependencies.add(dependencyPackage.id)
			}
		}

		graph.set(pkg.id, {
			pkg,
			dependencies: [...dependencies],
		})
	}

	return graph
}

function findCycles(graph) {
	const visiting = new Set()
	const visited = new Set()
	const stack = []
	const cycles = []

	function visit(packageId) {
		if (visited.has(packageId)) {
			return
		}

		if (visiting.has(packageId)) {
			const cycleStart = stack.indexOf(packageId)
			cycles.push([...stack.slice(cycleStart), packageId])
			return
		}

		visiting.add(packageId)
		stack.push(packageId)

		for (const dependencyId of graph.get(packageId)?.dependencies ?? []) {
			visit(dependencyId)
		}

		stack.pop()
		visiting.delete(packageId)
		visited.add(packageId)
	}

	for (const packageId of graph.keys()) {
		visit(packageId)
	}

	return cycles
}

const metadata = loadCargoMetadata()
const graph = buildWorkspaceGraph(metadata)
const cycles = findCycles(graph)

if (cycles.length > 0) {
	console.error('Rust workspace dependency cycle(s) detected:')
	for (const cycle of cycles) {
		console.error(
			`- ${cycle.map((packageId) => packageLabel(graph.get(packageId).pkg)).join(' -> ')}`,
		)
	}
	process.exit(1)
}

console.log(
	`No Rust workspace dependency cycles detected across ${graph.size} packages.`,
)
