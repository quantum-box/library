import type { ReactNode } from 'react'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DatabaseRecord, Status } from '../data/mock'
import { WorkflowView } from './WorkflowView'

interface TestNode {
  id: string
  data: { recordId: string; title: string }
}

interface TestEdge {
  id: string
  source: string
  target: string
  selected?: boolean
}

interface CapturedFlowProps {
  nodes: TestNode[]
  edges: TestEdge[]
  children?: ReactNode
  onNodeClick?: (event: React.MouseEvent, node: TestNode) => void
  onEdgeClick?: (event: React.MouseEvent, edge: TestEdge) => void
  onSelectionChange?: (selection: { nodes: TestNode[]; edges: TestEdge[] }) => void
}

const workflowMocks = vi.hoisted(() => ({
  flowProps: null as CapturedFlowProps | null,
  canvas: null as null | {
    databaseId: string
    selectedTemplateId: 'business-flow'
    nodes: Array<{
      id: string
      recordId: string
      templateId: 'business-flow'
      position: { x: number; y: number }
    }>
    edges: TestEdge[]
    updatedAt: string
  },
  saveWorkflowCanvas: vi.fn(),
  saveSyncedWorkflowCanvas: vi.fn(),
}))

vi.mock('@xyflow/react', () => ({
  Background: () => null,
  BackgroundVariant: { Dots: 'dots' },
  Controls: () => null,
  Handle: () => null,
  MiniMap: () => null,
  Position: { Left: 'left', Right: 'right' },
  ReactFlow: (props: CapturedFlowProps) => {
    workflowMocks.flowProps = props
    return (
      <div data-testid="mock-react-flow">
        {props.nodes.map((node) => (
          <button
            type="button"
            key={node.id}
            data-testid={`mock-workflow-node-${node.id}`}
            onClick={(event) => props.onNodeClick?.(event, node)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                props.onSelectionChange?.({ nodes: [node], edges: [] })
              }
            }}
          >
            {node.data.title}
          </button>
        ))}
        {props.edges.map((edge) => (
          <button
            type="button"
            key={edge.id}
            data-testid={`mock-workflow-edge-${edge.id}`}
            onClick={(event) => props.onEdgeClick?.(event, edge)}
          >
            Connection
          </button>
        ))}
        {props.children}
      </div>
    )
  },
  addEdge: (connection: TestEdge, edges: TestEdge[]) => [...edges, connection],
  applyEdgeChanges: (
    changes: Array<{ type: string; id?: string }>,
    edges: TestEdge[]
  ) =>
    edges.filter(
      (edge) => !changes.some((change) => change.type === 'remove' && change.id === edge.id)
    ),
  applyNodeChanges: (_changes: unknown, nodes: TestNode[]) => nodes,
}))

vi.mock('../lib/workflows/workflowDb', () => ({
  getWorkflowCanvas: vi.fn(async () => workflowMocks.canvas),
  saveWorkflowCanvas: workflowMocks.saveWorkflowCanvas.mockResolvedValue(undefined),
}))

vi.mock('../lib/workflows/workflowSync', () => ({
  getSyncedWorkflowCanvas: () => workflowMocks.canvas,
  saveSyncedWorkflowCanvas: workflowMocks.saveSyncedWorkflowCanvas,
  subscribeWorkflowCanvases: () => () => undefined,
}))

vi.mock('../lib/yjs/yjsProvider', () => ({
  initialSyncReady: Promise.resolve(),
}))

vi.mock('./DetailPanel', () => ({
  DetailPanel: ({ record }: { record: DatabaseRecord }) => (
    <div data-testid="workflow-detail-panel">{record.title}</div>
  ),
}))

function record(id: string, title: string, status: Status = 'todo'): DatabaseRecord {
  return {
    id,
    identifier: `DATA-${id}`,
    title,
    status,
    priority: 'none',
    assignee: null,
    labels: [],
    project: 'Test',
    createdAt: '2026-05-15T00:00:00.000Z',
    updatedAt: '2026-05-15T00:00:00.000Z',
    description: '',
  }
}

describe('WorkflowView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    workflowMocks.flowProps = null
    workflowMocks.canvas = null
    workflowMocks.saveWorkflowCanvas.mockResolvedValue(undefined)
  })

  it('shows every database item, including cancelled data, in the mobile-capable panel', async () => {
    const records = Array.from({ length: 41 }, (_, index) =>
      record(String(index), `Item ${index}`, index === 40 ? 'cancelled' : 'todo')
    )

    render(<WorkflowView databaseId="database-all" records={records} />)

    expect(screen.getAllByTestId('workflow-database-item')).toHaveLength(41)
    expect(screen.getByText('Item 40')).toBeInTheDocument()
    expect(screen.getByTestId('workflow-elements-panel')).not.toHaveClass('hidden')
    await waitFor(() => expect(workflowMocks.flowProps).not.toBeNull())
  })

  it('paginates more than 100 matching records without exceeding the DOM bound', async () => {
    const records = Array.from({ length: 160 }, (_, index) =>
      record(String(index), 'Shared result')
    )

    render(<WorkflowView databaseId="database-large" records={records} />)

    expect(screen.getAllByTestId('workflow-database-item')).toHaveLength(100)
    expect(screen.getByTestId('workflow-item-count')).toHaveTextContent(
      'Showing 1-100 of 160 matching items',
    )
    expect(screen.queryByText('DATA-159')).not.toBeInTheDocument()

    fireEvent.change(screen.getByRole('searchbox', { name: 'Search Library data' }), {
      target: { value: 'Shared result' },
    })

    expect(screen.getAllByTestId('workflow-database-item')).toHaveLength(100)
    fireEvent.click(screen.getByRole('button', { name: 'Next Library data page' }))

    expect(screen.getAllByTestId('workflow-database-item')).toHaveLength(60)
    expect(screen.getByText('DATA-159')).toBeInTheDocument()
    expect(screen.queryByText('DATA-0')).not.toBeInTheDocument()
    expect(screen.getByTestId('workflow-item-count')).toHaveTextContent(
      'Showing 101-160 of 160 matching items',
    )

    fireEvent.change(screen.getByRole('searchbox', { name: 'Search Library data' }), {
      target: { value: 'DATA-0' },
    })
    expect(screen.getAllByTestId('workflow-database-item')).toHaveLength(1)
    expect(screen.getByText('DATA-0')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Next Library data page' })).not.toBeInTheDocument()
  })

  it('opens details from single click and keyboard selection without an empty panel', async () => {
    const first = record('one', 'First node')
    workflowMocks.canvas = {
      databaseId: 'database-one',
      selectedTemplateId: 'business-flow',
      nodes: [
        {
          id: 'node-one',
          recordId: first.id,
          templateId: 'business-flow',
          position: { x: 0, y: 0 },
        },
      ],
      edges: [],
      updatedAt: '2026-05-15T00:00:00.000Z',
    }

    render(<WorkflowView databaseId="database-one" records={[first]} />)

    expect(screen.queryByTestId('workflow-detail-panel')).not.toBeInTheDocument()
    const node = await screen.findByTestId('mock-workflow-node-node-one')
    fireEvent.click(node)
    expect(screen.getByTestId('workflow-detail-panel')).toHaveTextContent('First node')

    fireEvent.keyDown(node, { key: 'Enter' })
    expect(screen.getByTestId('workflow-detail-panel')).toHaveTextContent('First node')
  })

  it('keeps selected details open when records hydrate for the same database', async () => {
    const first = record('one', 'Snapshot title')
    workflowMocks.canvas = {
      databaseId: 'database-one',
      selectedTemplateId: 'business-flow',
      nodes: [
        {
          id: 'node-one',
          recordId: first.id,
          templateId: 'business-flow',
          position: { x: 0, y: 0 },
        },
      ],
      edges: [],
      updatedAt: '2026-05-15T00:00:00.000Z',
    }

    const { rerender } = render(
      <WorkflowView databaseId="database-one" records={[first]} />
    )

    fireEvent.click(await screen.findByTestId('mock-workflow-node-node-one'))
    expect(screen.getByTestId('workflow-detail-panel')).toHaveTextContent('Snapshot title')

    rerender(
      <WorkflowView
        databaseId="database-one"
        records={[{ ...first, title: 'Hydrated title' }]}
      />
    )

    await waitFor(() => {
      expect(screen.getByTestId('workflow-detail-panel')).toHaveTextContent('Hydrated title')
    })
  })

  it('provides a visible delete affordance for a selected connection', async () => {
    const first = record('one', 'First node')
    const second = record('two', 'Second node')
    workflowMocks.canvas = {
      databaseId: 'database-edge',
      selectedTemplateId: 'business-flow',
      nodes: [
        {
          id: 'node-one',
          recordId: first.id,
          templateId: 'business-flow',
          position: { x: 0, y: 0 },
        },
        {
          id: 'node-two',
          recordId: second.id,
          templateId: 'business-flow',
          position: { x: 300, y: 0 },
        },
      ],
      edges: [{ id: 'edge-one-two', source: 'node-one', target: 'node-two' }],
      updatedAt: '2026-05-15T00:00:00.000Z',
    }

    render(<WorkflowView databaseId="database-edge" records={[first, second]} />)

    fireEvent.click(await screen.findByTestId('mock-workflow-edge-edge-one-two'))
    fireEvent.click(screen.getByTestId('workflow-delete-edge'))

    await waitFor(() => {
      expect(screen.queryByTestId('mock-workflow-edge-edge-one-two')).not.toBeInTheDocument()
    })
  })
})
