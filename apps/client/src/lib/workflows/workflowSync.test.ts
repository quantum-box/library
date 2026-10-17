import { beforeEach, describe, expect, it, vi } from 'vitest'
const { canvases } = vi.hoisted(() => ({ canvases: new Map<string, string>() }))
vi.mock('../yjs/yjsProvider', () => ({
  workflowCanvasesMap: canvases,
  ydoc: { transact: (action: () => void) => action() },
}))
import { getSyncedWorkflowCanvas, saveSyncedWorkflowCanvas } from './workflowSync'

describe('workflow settings synchronization', () => {
  beforeEach(() => canvases.clear())
  it('loads legacy canvases and round-trips new view settings', () => {
    canvases.set('old', JSON.stringify({ databaseId: 'old', selectedTemplateId: 'business-flow', nodes: [], edges: [] }))
    expect(getSyncedWorkflowCanvas('old')?.settings).toEqual({ visiblePropertyIds: [], relationPropertyId: '', collapsedNodeIds: [] })
    const settings = { visiblePropertyIds: ['target'], relationPropertyId: 'children', collapsedNodeIds: ['root'] }
    saveSyncedWorkflowCanvas({ databaseId: 'new', selectedTemplateId: 'kpi-tree', nodes: [], edges: [], settings })
    expect(getSyncedWorkflowCanvas('new')?.settings).toEqual(settings)
  })
})
