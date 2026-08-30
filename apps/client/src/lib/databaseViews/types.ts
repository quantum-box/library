import type { Priority, Status } from '../../data/mock'

export type DatabaseViewType = 'table' | 'board' | 'workflow' | 'timeline'

export type RecordPropertyKey =
  | 'identifier'
  | 'status'
  | 'priority'
  | 'title'
  | 'assignee'
  | 'labels'
  | 'project'
  | 'updatedAt'

/** Timestamp a timeline bar starts from. Every bar ends at `updatedAt`. */
export type TimelineDateField = 'createdAt' | 'updatedAt'

export type TimelineScale = 'day' | 'week' | 'month'

export interface DatabaseViewFilters {
  search: string
  status?: Status
  priority?: Priority
  assignee?: string
  labels: string[]
  project?: string
}

export interface DatabaseViewSorting {
  id: RecordPropertyKey
  desc: boolean
}

export interface DatabaseViewBoardSettings {
  compact: boolean
}

export interface DatabaseViewTimelineSettings {
  startField: TimelineDateField
  scale: TimelineScale
}

export interface DatabaseViewDefinition {
  id: string
  databaseId: string
  name: string
  type: DatabaseViewType
  filters: DatabaseViewFilters
  sorting: DatabaseViewSorting | null
  visibleProperties: RecordPropertyKey[]
  board: DatabaseViewBoardSettings
  timeline: DatabaseViewTimelineSettings
  workflowCanvasKey: string
  order: number
  createdAt: string
  updatedAt: string
}
