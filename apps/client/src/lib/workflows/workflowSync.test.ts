import { beforeEach, describe, expect, it, vi } from 'vitest'
const { canvases } = vi.hoisted(() => ({ canvases: new Map<string, string>() }))
vi.mock('../yjs/yjsProvider', () => ({
  workflowCanvasesMap: canvases,
  ydoc: { transact: (action: () => void) => action() },
}))
import { getSyncedWorkflowCanvas, saveSyncedWorkflowCanvas } from './workflowSync'

describe('workflow settings synchronization', () => {
  beforeEach(() => canvases.clear())
  it('maps the former preview template to a generic tree without losing nodes', () => {
    canvases.set('legacy-tree', JSON.stringify({
      databaseId: 'legacy-tree', selectedTemplateId: 'kpi-tree',
      nodes: [{ id: 'node', recordId: 'data', templateId: 'kpi-tree', position: { x: 10, y: 20 } }], edges: [],
    }))
    const canvas = getSyncedWorkflowCanvas('legacy-tree')
    expect(canvas?.selectedTemplateId).toBe('tree')
    expect(canvas?.nodes).toEqual([{ id: 'node', recordId: 'data', templateId: 'tree', position: { x: 10, y: 20 } }])
  })
  it('loads legacy canvases and round-trips new view settings', () => {
    canvases.set('old', JSON.stringify({ databaseId: 'old', selectedTemplateId: 'business-flow', nodes: [], edges: [] }))
    expect(getSyncedWorkflowCanvas('old')?.settings).toEqual({ visiblePropertyIds: [], relationPropertyId: '', collapsedNodeIds: [] })
    const settings = { visiblePropertyIds: ['target'], relationPropertyId: 'children', collapsedNodeIds: ['root'] }
    saveSyncedWorkflowCanvas({ databaseId: 'new', selectedTemplateId: 'tree', nodes: [], edges: [], settings })
    expect(getSyncedWorkflowCanvas('new')?.settings).toEqual(settings)
  })
})
