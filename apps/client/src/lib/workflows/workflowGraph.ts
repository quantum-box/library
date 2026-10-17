import type { Edge, XYPosition } from '@xyflow/react'

/** Accept the former preview template id without exposing domain terminology. */
export function normalizeWorkflowTemplateId(value: unknown): 'business-flow' | 'tree' {
  return value === 'tree' || value === 'kpi-tree' ? 'tree' : 'business-flow'
}

export interface WorkflowSettings {
  visiblePropertyIds: string[]
  relationPropertyId: string
  collapsedNodeIds: string[]
}

export function normalizeWorkflowSettings(value?: Partial<WorkflowSettings> | null): WorkflowSettings {
  const strings = (input: unknown) => Array.isArray(input)
    ? [...new Set(input.filter((item): item is string => typeof item === 'string'))] : []
  return {
    visiblePropertyIds: strings(value?.visiblePropertyIds),
    relationPropertyId: typeof value?.relationPropertyId === 'string' ? value.relationPropertyId : '',
    collapsedNodeIds: strings(value?.collapsedNodeIds),
  }
}

/** True when adding source → target would introduce a cycle. */
export function wouldCreateCycle(edges: Pick<Edge, 'source' | 'target'>[], source: string, target: string): boolean {
  const pending = [target]
  const visited = new Set<string>()
  while (pending.length) {
    const id = pending.pop()!
    if (id === source) return true
    if (visited.has(id)) continue
    visited.add(id)
    pending.push(...edges.filter((edge) => edge.source === id).map((edge) => edge.target))
  }
  return false
}

/** Hide descendants of collapsed nodes; retain the collapsed cards themselves. */
export function hiddenWorkflowNodes(edges: Pick<Edge, 'source' | 'target'>[], collapsed: string[]): Set<string> {
  const hidden = new Set<string>()
  for (const root of collapsed) {
    const visited = new Set([root])
    const pending = edges.filter((edge) => edge.source === root).map((edge) => edge.target)
    while (pending.length) {
      const id = pending.pop()!
      if (visited.has(id)) continue
      visited.add(id)
      hidden.add(id)
      pending.push(...edges.filter((edge) => edge.source === id).map((edge) => edge.target))
    }
  }
  return hidden
}

/** Stable layered layout. Cyclic components remain visible in a final column. */
export function layoutWorkflowNodes(nodes: { id: string }[], edges: Pick<Edge, 'source' | 'target'>[], rowHeight = 220): Map<string, XYPosition> {
  const ids = new Set(nodes.map((node) => node.id))
  const links = edges.filter((edge) => ids.has(edge.source) && ids.has(edge.target))
  const incoming = new Map(nodes.map((node) => [node.id, links.filter((edge) => edge.target === node.id).length]))
  const depth = new Map(nodes.map((node) => [node.id, 0]))
  const queue = nodes.filter((node) => incoming.get(node.id) === 0).map((node) => node.id)
  const visited = new Set<string>()
  for (let index = 0; index < queue.length; index++) {
    const id = queue[index]
    visited.add(id)
    for (const edge of links.filter((edge) => edge.source === id)) {
      depth.set(edge.target, Math.max(depth.get(edge.target)!, depth.get(id)! + 1))
      incoming.set(edge.target, incoming.get(edge.target)! - 1)
      if (incoming.get(edge.target) === 0) queue.push(edge.target)
    }
  }
  const cycleDepth = visited.size ? Math.max(...[...visited].map((id) => depth.get(id)!)) + 1 : 0
  const rows = new Map<number, number>()
  return new Map(nodes.map(({ id }) => {
    const column = visited.has(id) ? depth.get(id)! : cycleDepth
    const row = rows.get(column) ?? 0
    rows.set(column, row + 1)
    return [id, { x: 80 + column * 340, y: 80 + row * rowHeight }]
  }))
}
