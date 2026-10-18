import { describe, expect, it } from 'vitest'
import { hiddenWorkflowNodes, layoutWorkflowNodes, normalizeWorkflowSettings, wouldCreateCycle } from './workflowGraph'

const nodes = ['root', 'left', 'right', 'leaf', 'isolated'].map((id) => ({ id }))
const edges = [
  { source: 'root', target: 'left' },
  { source: 'root', target: 'right' },
  { source: 'left', target: 'leaf' },
  { source: 'right', target: 'leaf' },
]

describe('workflow graph', () => {
  it('rejects indirect cycles while allowing shared descendants', () => {
    expect(wouldCreateCycle(edges, 'leaf', 'root')).toBe(true)
    expect(wouldCreateCycle(edges, 'root', 'root')).toBe(true)
    expect(wouldCreateCycle(edges, 'left', 'right')).toBe(false)
  })
  it('lays out a DAG by longest path and keeps disconnected nodes', () => {
    const positions = layoutWorkflowNodes(nodes, edges)
    expect(positions.size).toBe(nodes.length)
    expect(positions.get('leaf')!.x).toBeGreaterThan(positions.get('left')!.x)
    expect(positions.get('left')!.x).toBe(positions.get('right')!.x)
    expect(positions.get('left')!.y).not.toBe(positions.get('right')!.y)
    expect(layoutWorkflowNodes(nodes, edges)).toEqual(positions)
  })
  it('terminates and retains every node for legacy graphs containing a cycle', () => {
    const cyclic = [...edges, { source: 'leaf', target: 'root' }]
    expect(layoutWorkflowNodes(nodes, cyclic).size).toBe(nodes.length)
    expect(hiddenWorkflowNodes(cyclic, ['root'])).toEqual(new Set(['left', 'right', 'leaf']))
  })
  it('restores a legacy canvas with defaults and filters malformed settings', () => {
    expect(normalizeWorkflowSettings()).toEqual({ visiblePropertyIds: [], collapsedNodeIds: [], relationPropertyId: '' })
    expect(normalizeWorkflowSettings({ visiblePropertyIds: ['a', 'a'], relationPropertyId: 'parent' })).toEqual({ visiblePropertyIds: ['a'], relationPropertyId: 'parent', collapsedNodeIds: [] })
  })
})
