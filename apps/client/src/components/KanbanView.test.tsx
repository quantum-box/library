import type { ReactNode } from 'react'
import { act, render, screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DatabaseRecord, Status } from '../data/mock'
import { KanbanView } from './KanbanView'

interface CapturedDndProps {
  children?: ReactNode
  onDragStart: (event: { active: { id: string } }) => void
  onDragOver: (event: {
    active: { id: string }
    over: { id: string; data: { current?: { status?: Status } } } | null
  }) => void
  onDragEnd: (event: {
    active: { id: string }
    over: { id: string; data: { current?: { status?: Status } } } | null
  }) => void
  onDragCancel: () => void
}

const dndCapture = vi.hoisted(() => ({
  current: null as CapturedDndProps | null,
}))

vi.mock('@dnd-kit/core', () => ({
  DndContext: (props: CapturedDndProps) => {
    dndCapture.current = props
    return <>{props.children}</>
  },
  DragOverlay: ({ children }: { children?: ReactNode }) => <>{children}</>,
  KeyboardSensor: function KeyboardSensor() {},
  PointerSensor: function PointerSensor() {},
  closestCorners: vi.fn(),
  useDroppable: () => ({ setNodeRef: vi.fn(), isOver: false }),
  useSensor: () => ({}),
  useSensors: () => [],
}))

vi.mock('@dnd-kit/sortable', () => ({
  SortableContext: ({ children }: { children?: ReactNode }) => <>{children}</>,
  useSortable: () => ({
    attributes: {},
    listeners: {},
    setNodeRef: vi.fn(),
    transform: null,
    transition: undefined,
    isDragging: false,
  }),
  verticalListSortingStrategy: {},
}))

vi.mock('@dnd-kit/utilities', () => ({
  CSS: { Transform: { toString: () => undefined } },
}))

function record(id: string, title: string, status: Status): DatabaseRecord {
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

describe('KanbanView', () => {
  beforeEach(() => {
    dndCapture.current = null
  })

  it('renders cancelled data in a real workflow column', () => {
    render(
      <KanbanView
        records={[record('cancelled', 'Cancelled item', 'cancelled')]}
        selectedRecordId={null}
        onSelectRecord={() => undefined}
        onMoveRecord={() => undefined}
      />
    )

    expect(
      within(screen.getByTestId('kanban-column-cancelled')).getByText('Cancelled item')
    ).toBeInTheDocument()
  })

  it('uses local drag preview and persists the status exactly once on drop', () => {
    const onMoveRecord = vi.fn()
    render(
      <KanbanView
        records={[record('one', 'Move me', 'todo')]}
        selectedRecordId={null}
        onSelectRecord={() => undefined}
        onMoveRecord={onMoveRecord}
      />
    )

    act(() => {
      dndCapture.current?.onDragStart({ active: { id: 'one' } })
    })
    act(() => {
      dndCapture.current?.onDragOver({
        active: { id: 'one' },
        over: { id: 'column-cancelled', data: { current: { status: 'cancelled' } } },
      })
    })

    expect(onMoveRecord).not.toHaveBeenCalled()
    expect(
      within(screen.getByTestId('kanban-column-cancelled')).getByText('Move me')
    ).toBeInTheDocument()

    act(() => {
      dndCapture.current?.onDragEnd({
        active: { id: 'one' },
        over: { id: 'column-cancelled', data: { current: { status: 'cancelled' } } },
      })
    })

    expect(onMoveRecord).toHaveBeenCalledTimes(1)
    expect(onMoveRecord).toHaveBeenCalledWith('one', 'cancelled')
  })

  it('rolls the local preview back without persistence when dragging is cancelled', () => {
    const onMoveRecord = vi.fn()
    render(
      <KanbanView
        records={[record('one', 'Keep me', 'todo')]}
        selectedRecordId={null}
        onSelectRecord={() => undefined}
        onMoveRecord={onMoveRecord}
      />
    )

    act(() => {
      dndCapture.current?.onDragStart({ active: { id: 'one' } })
      dndCapture.current?.onDragOver({
        active: { id: 'one' },
        over: { id: 'column-cancelled', data: { current: { status: 'cancelled' } } },
      })
    })
    act(() => {
      dndCapture.current?.onDragCancel()
    })

    expect(onMoveRecord).not.toHaveBeenCalled()
    expect(
      within(screen.getByTestId('kanban-column-todo')).getByText('Keep me')
    ).toBeInTheDocument()
  })
})
