/* eslint-disable react-hooks/set-state-in-effect */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { DragEvent, MouseEvent } from 'react'
import {
  Background,
  BackgroundVariant,
  Controls,
  Handle,
  MiniMap,
  Position,
  ReactFlow,
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
  type NodeProps,
  type ReactFlowInstance,
} from '@xyflow/react'
import { ListPlus, PanelLeftClose, Search, Trash2 } from 'lucide-react'
import '@xyflow/react/dist/style.css'
import { priorityConfig, statusConfig, type DatabaseRecord } from '../data/mock'
import {
  getWorkflowCanvas,
  saveWorkflowCanvas,
  type PersistedWorkflowCanvasNode,
  type WorkflowCanvas,
  type WorkflowCanvasTemplateId,
} from '../lib/workflows/workflowDb'
import {
  getSyncedWorkflowCanvas,
  saveSyncedWorkflowCanvas,
  subscribeWorkflowCanvases,
} from '../lib/workflows/workflowSync'
import { initialSyncReady } from '../lib/yjs/yjsProvider'
import { DetailPanel } from './DetailPanel'

type WorkflowTemplateId = WorkflowCanvasTemplateId

type WorkflowTemplate = {
  id: WorkflowTemplateId
  label: string
  description: string
  canvasHint: string
}

type WorkflowNodeData = {
  recordId: string
  identifier: string
  title: string
  description: string
  status: DatabaseRecord['status']
  priority: DatabaseRecord['priority']
  accent: string
  templateId: WorkflowTemplateId
}

type WorkflowRecordNode = Node<WorkflowNodeData, 'workflowRecord'>

const workflowTemplates: WorkflowTemplate[] = [
  {
    id: 'business-flow',
    label: 'Business Flow',
    description: 'Arrange database items as process steps, decisions, handoffs, and outcomes.',
    canvasHint: 'Use the database items as the real steps in the operation.',
  },
  {
    id: 'kpi-tree',
    label: 'KPI Tree',
    description: 'Arrange database items as goals, KPIs, drivers, initiatives, and guardrails.',
    canvasHint: 'Use the database items as the measurable pieces of the tree.',
  },
]

function WorkflowRecordNode({ data, selected }: NodeProps<WorkflowRecordNode>) {
  return (
    <div
      className={`w-[260px] rounded-lg border bg-surface p-3 shadow-soft transition-colors ${
        selected ? 'border-accent' : 'border-border'
      }`}
      data-testid="workflow-node-record"
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!-left-2 !z-20 !h-4 !w-4 !border-2 !border-panel !bg-accent"
        data-testid="workflow-handle-target"
      />
      <div className="mb-2 flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-[10px] font-medium text-subtle">{data.identifier}</div>
          <div className="mt-0.5 line-clamp-2 text-sm font-semibold text-foreground">
            {data.title}
          </div>
        </div>
        <span
          className="shrink-0 rounded bg-canvas px-2 py-1 text-[10px] font-medium"
          style={{ color: data.accent }}
        >
          {priorityConfig[data.priority].label}
        </span>
      </div>
      <div className="line-clamp-2 text-xs leading-relaxed text-subtle">
        {data.description}
      </div>
      <div className="mt-3 flex items-center justify-between gap-2 text-[10px] text-subtle">
        <span style={{ color: statusConfig[data.status].color }}>
          {statusConfig[data.status].icon} {statusConfig[data.status].label}
        </span>
        <span>{data.templateId === 'kpi-tree' ? 'KPI tree item' : 'Flow item'}</span>
      </div>
      <Handle
        type="source"
        position={Position.Right}
        className="!-right-2 !z-20 !h-4 !w-4 !border-2 !border-panel !bg-accent"
        data-testid="workflow-handle-source"
      />
    </div>
  )
}

const nodeTypes = {
  workflowRecord: WorkflowRecordNode,
}

const WORKFLOW_SYNC_THROTTLE_MS = 80
const WORKFLOW_CACHE_WRITE_DELAY_MS = 220
const WORKFLOW_ITEM_PAGE_SIZE = 100

type PendingWorkflowSync = {
  databaseId: string
  selectedTemplateId: WorkflowTemplateId
  nodes: PersistedWorkflowCanvasNode[]
  edges: Edge[]
  signature: string
  countSignature: string
}

function createRecordNode(
  record: DatabaseRecord,
  templateId: WorkflowTemplateId,
  position: { x: number; y: number },
  index: number,
  nodeId?: string
): WorkflowRecordNode {
  return {
    id: nodeId ?? `workflow-record-${record.id}-${index}`,
    type: 'workflowRecord',
    position,
    data: {
      recordId: record.id,
      identifier: record.identifier,
      title: record.title,
      description: record.description,
      status: record.status,
      priority: record.priority,
      accent: priorityConfig[record.priority].color,
      templateId,
    },
  }
}

function isWorkflowTemplateId(value: unknown): value is WorkflowTemplateId {
  return value === 'business-flow' || value === 'kpi-tree'
}

function isRecordSnapshot(
  value: PersistedWorkflowCanvasNode['recordSnapshot']
): value is NonNullable<PersistedWorkflowCanvasNode['recordSnapshot']> {
  return Boolean(
    value &&
      value.id &&
      value.identifier &&
      value.title &&
      value.status in statusConfig &&
      value.priority in priorityConfig
  )
}

function getNodeSequenceValue(nodeId: string) {
  const match = nodeId.match(/-(\d+)$/)
  return match ? Number(match[1]) : 0
}

export function WorkflowView({
  databaseId,
  records,
  onUpdateRecord,
  onDeleteRecord,
}: {
  databaseId: string
  records: DatabaseRecord[]
  onUpdateRecord?: (recordId: string, field: keyof DatabaseRecord, value: string) => void
  onDeleteRecord?: (recordId: string) => void
}) {
  const wrapperRef = useRef<HTMLDivElement | null>(null)
  const [selectedTemplateId, setSelectedTemplateId] =
    useState<WorkflowTemplateId>('business-flow')
  const [itemsPanelOpen, setItemsPanelOpen] = useState(true)
  const [nodes, setNodes] = useState<WorkflowRecordNode[]>([])
  const [edges, setEdges] = useState<Edge[]>([])
  const [reactFlowInstance, setReactFlowInstance] =
    useState<ReactFlowInstance<WorkflowRecordNode, Edge> | null>(null)
  const [loadedDatabaseId, setLoadedDatabaseId] = useState<string | null>(null)
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved'>('idle')
  const [savedSignature, setSavedSignature] = useState('')
  const [savedCountSignature, setSavedCountSignature] = useState('')
  const [previewRecordId, setPreviewRecordId] = useState<string | null>(null)
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null)
  const [recordQuery, setRecordQuery] = useState('')
  const [recordPage, setRecordPage] = useState(0)
  const nodeSequence = useRef(0)
  const hasLocalCanvasChanges = useRef(false)
  const skipNextSyncWrite = useRef(false)
  const skipInitialEmptySyncWrite = useRef(false)
  const pendingSync = useRef<PendingWorkflowSync | null>(null)
  const syncThrottleTimer = useRef<number | null>(null)
  const lastSyncAt = useRef(0)
  const localNodeDragActive = useRef(false)
  const workflowLoaded = loadedDatabaseId === databaseId

  const selectedTemplate = workflowTemplates.find(
    (template) => template.id === selectedTemplateId
  ) ?? workflowTemplates[0]

  const normalizedRecordQuery = recordQuery.trim().toLowerCase()
  const matchingRecords = useMemo(() => {
    if (!normalizedRecordQuery) return records

    return records.filter((record) => [
      record.identifier,
      record.title,
      statusConfig[record.status].label,
      priorityConfig[record.priority].label,
    ].some((value) => value.toLowerCase().includes(normalizedRecordQuery)))
  }, [normalizedRecordQuery, records])
  const recordPageCount = Math.max(
    1,
    Math.ceil(matchingRecords.length / WORKFLOW_ITEM_PAGE_SIZE)
  )
  const visibleRecordPage = Math.min(recordPage, recordPageCount - 1)
  const visibleRecords = useMemo(
    () => matchingRecords.slice(
      visibleRecordPage * WORKFLOW_ITEM_PAGE_SIZE,
      (visibleRecordPage + 1) * WORKFLOW_ITEM_PAGE_SIZE
    ),
    [matchingRecords, visibleRecordPage]
  )
  const visibleRecordStart = matchingRecords.length === 0
    ? 0
    : visibleRecordPage * WORKFLOW_ITEM_PAGE_SIZE + 1
  const visibleRecordEnd = matchingRecords.length === 0
    ? 0
    : visibleRecordStart + visibleRecords.length - 1
  const previewRecord = useMemo<DatabaseRecord | null>(() => {
    if (!previewRecordId) return null

    const liveRecord = records.find((record) => record.id === previewRecordId)
    if (liveRecord) return liveRecord

    const node = nodes.find((candidate) => candidate.data.recordId === previewRecordId)
    if (!node) return null

    return {
      id: node.data.recordId,
      identifier: node.data.identifier,
      title: node.data.title,
      description: node.data.description,
      status: node.data.status,
      priority: node.data.priority,
      assignee: null,
      labels: [],
      project: '',
      updatedAt: new Date().toISOString(),
      createdAt: new Date().toISOString(),
    }
  }, [nodes, previewRecordId, records])
  const canvasCountSignature = useMemo(
    () => `${selectedTemplateId}:${nodes.length}:${edges.length}`,
    [edges.length, nodes.length, selectedTemplateId]
  )
  const renderedEdges = useMemo(
    () => edges.map((edge) => ({ ...edge, selected: edge.id === selectedEdgeId })),
    [edges, selectedEdgeId]
  )
  const canvasSignature = useMemo(
    () =>
      JSON.stringify({
        selectedTemplateId,
        nodes: nodes.map((node) => ({
          id: node.id,
          recordId: node.data.recordId,
          templateId: node.data.templateId,
          x: Math.round(node.position.x),
          y: Math.round(node.position.y),
        })),
        edges: edges.map((edge) => ({
          id: edge.id,
          source: edge.source,
          target: edge.target,
          sourceHandle: edge.sourceHandle,
          targetHandle: edge.targetHandle,
        })),
      }),
    [edges, nodes, selectedTemplateId]
  )
  const persistedNodes = useMemo<PersistedWorkflowCanvasNode[]>(
    () =>
      nodes.map((node) => ({
        id: node.id,
        recordId: node.data.recordId,
        templateId: node.data.templateId,
        position: node.position,
        recordSnapshot: {
          id: node.data.recordId,
          identifier: node.data.identifier,
          title: node.data.title,
          description: node.data.description,
          status: node.data.status,
          priority: node.data.priority,
        },
      })),
    [nodes]
  )

  const flushWorkflowSync = useCallback(() => {
    const payload = pendingSync.current
    if (!payload) return

    pendingSync.current = null
    lastSyncAt.current = Date.now()

    try {
      saveSyncedWorkflowCanvas({
        databaseId: payload.databaseId,
        selectedTemplateId: payload.selectedTemplateId,
        nodes: payload.nodes,
        edges: payload.edges,
      })
    } catch (error) {
      console.warn('Failed to sync workflow canvas', error)
    }
  }, [])

  const applyWorkflowCanvas = useCallback(
    (canvas: WorkflowCanvas) => {
      const nextNodes = canvas.nodes.flatMap((node) => {
        const record =
          records.find((item) => item.id === node.recordId) ??
          (isRecordSnapshot(node.recordSnapshot)
            ? {
                ...node.recordSnapshot,
                assignee: null,
                labels: [],
                project: '',
                createdAt: canvas.updatedAt,
                updatedAt: canvas.updatedAt,
              }
            : null)
        if (!record || !isWorkflowTemplateId(node.templateId)) return []
        return [
          createRecordNode(record, node.templateId, node.position, 0, node.id),
        ]
      })

      setNodes((current) => {
        const currentById = new Map(current.map((node) => [node.id, node]))
        return nextNodes.map((nextNode) => {
          const existing = currentById.get(nextNode.id)
          if (!existing) return nextNode

          return {
            ...existing,
            type: nextNode.type,
            position: nextNode.position,
            data: nextNode.data,
          }
        })
      })
      setEdges(canvas.edges)
      setSelectedTemplateId(canvas.selectedTemplateId)
      nodeSequence.current = nextNodes.reduce(
        (max, node) => Math.max(max, getNodeSequenceValue(node.id)),
        0
      )
    },
    [records]
  )

  useEffect(() => {
    hasLocalCanvasChanges.current = false
    setLoadedDatabaseId(null)
    setPreviewRecordId(null)
    setSelectedEdgeId(null)
    setRecordQuery('')
    setRecordPage(0)
  }, [databaseId])

  useEffect(() => {
    let cancelled = false

    void initialSyncReady
      .then(() => {
        const syncedCanvas = getSyncedWorkflowCanvas(databaseId)
        if (cancelled) return

        if (syncedCanvas) {
          if (!hasLocalCanvasChanges.current) {
            skipNextSyncWrite.current = true
            applyWorkflowCanvas(syncedCanvas)
          }
          setLoadedDatabaseId(databaseId)
          return
        }

        if (hasLocalCanvasChanges.current) {
          setLoadedDatabaseId(databaseId)
          return
        }

        setNodes([])
        setEdges([])
        nodeSequence.current = 0
        setSelectedTemplateId('business-flow')
        skipInitialEmptySyncWrite.current = true
        setLoadedDatabaseId(databaseId)

        void getWorkflowCanvas(databaseId).then((persistedCanvas) => {
          if (cancelled) return
          if (hasLocalCanvasChanges.current) {
            setLoadedDatabaseId(databaseId)
            return
          }
          if (!persistedCanvas) {
            return
          }
          const latestSyncedCanvas = getSyncedWorkflowCanvas(databaseId)
          if (latestSyncedCanvas) {
            skipNextSyncWrite.current = true
            applyWorkflowCanvas(latestSyncedCanvas)
            setLoadedDatabaseId(databaseId)
            return
          }
          skipNextSyncWrite.current = true
          applyWorkflowCanvas(persistedCanvas)
          setLoadedDatabaseId(databaseId)
        }).catch(() => {
          if (cancelled) return
          setNodes([])
          setEdges([])
          nodeSequence.current = 0
          setSelectedTemplateId('business-flow')
          skipNextSyncWrite.current = true
          setLoadedDatabaseId(databaseId)
        })
      })
      .catch(() => {
        if (cancelled) return
        setNodes([])
        setEdges([])
        nodeSequence.current = 0
        setSelectedTemplateId('business-flow')
        setLoadedDatabaseId(databaseId)
      })

    return () => {
      cancelled = true
    }
  }, [applyWorkflowCanvas, databaseId])

  useEffect(() => {
    function applyCurrentSyncedCanvas() {
      const syncedCanvas = getSyncedWorkflowCanvas(databaseId)
      if (!syncedCanvas) return
      if (localNodeDragActive.current) return

      skipNextSyncWrite.current = true
      applyWorkflowCanvas(syncedCanvas)
      setLoadedDatabaseId(databaseId)
    }

    applyCurrentSyncedCanvas()
    return subscribeWorkflowCanvases(applyCurrentSyncedCanvas)
  }, [applyWorkflowCanvas, databaseId])

  useEffect(() => {
    if (loadedDatabaseId !== databaseId) return

    setSaveStatus('saving')
    const nextSignature = canvasSignature
    const nextCountSignature = canvasCountSignature
    const shouldSkipSyncWrite = skipNextSyncWrite.current
    const shouldSkipInitialEmptySyncWrite =
      skipInitialEmptySyncWrite.current && !hasLocalCanvasChanges.current
    skipNextSyncWrite.current = false
    skipInitialEmptySyncWrite.current = false

    if (shouldSkipSyncWrite || shouldSkipInitialEmptySyncWrite) {
      setSavedSignature(nextSignature)
      setSavedCountSignature(nextCountSignature)
      setSaveStatus('saved')
    } else {
      pendingSync.current = {
        databaseId,
        selectedTemplateId,
        nodes: persistedNodes,
        edges,
        signature: nextSignature,
        countSignature: nextCountSignature,
      }

      const elapsed = Date.now() - lastSyncAt.current
      if (elapsed >= WORKFLOW_SYNC_THROTTLE_MS && syncThrottleTimer.current === null) {
        flushWorkflowSync()
      } else if (syncThrottleTimer.current === null) {
        syncThrottleTimer.current = window.setTimeout(() => {
          syncThrottleTimer.current = null
          flushWorkflowSync()
        }, Math.max(0, WORKFLOW_SYNC_THROTTLE_MS - elapsed))
      }
    }

    let active = true
    const cacheTimer = window.setTimeout(() => {
      void saveWorkflowCanvas({
        databaseId,
        selectedTemplateId,
        nodes: persistedNodes,
        edges,
      })
        .then(() => {
          if (!active) return
          setSavedSignature(nextSignature)
          setSavedCountSignature(nextCountSignature)
          setSaveStatus('saved')
        })
        .catch(() => {
          // PGlite is a local cache for fast reload; Yjs remains the sync source.
        })
    }, WORKFLOW_CACHE_WRITE_DELAY_MS)

    return () => {
      active = false
      window.clearTimeout(cacheTimer)
    }
  }, [
    canvasCountSignature,
    canvasSignature,
    databaseId,
    edges,
    flushWorkflowSync,
    loadedDatabaseId,
    persistedNodes,
    selectedTemplateId,
  ])

  useEffect(() => {
    return () => {
      if (syncThrottleTimer.current !== null) {
        window.clearTimeout(syncThrottleTimer.current)
      }
    }
  }, [])

  const addRecord = useCallback(
    (record: DatabaseRecord, position?: { x: number; y: number }) => {
      hasLocalCanvasChanges.current = true
      nodeSequence.current += 1
      const nextPosition =
        position ?? {
          x: 120 + (nodeSequence.current % 3) * 300,
          y: 120 + Math.floor(nodeSequence.current / 3) * 180,
        }

      setNodes((current) => [
        ...current,
        createRecordNode(record, selectedTemplateId, nextPosition, nodeSequence.current),
      ])
    },
    [selectedTemplateId]
  )

  const handleDragStart = (event: DragEvent, record: DatabaseRecord) => {
    event.dataTransfer.setData('application/photon-workflow-record', record.id)
    event.dataTransfer.effectAllowed = 'move'
  }

  const handleDrop = useCallback(
    (event: DragEvent) => {
      event.preventDefault()

      const recordId = event.dataTransfer.getData('application/photon-workflow-record')
      const record = records.find((item) => item.id === recordId)
      if (!record || !reactFlowInstance || !wrapperRef.current) return

      const position = reactFlowInstance.screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      })

      addRecord(record, position)
    },
    [addRecord, reactFlowInstance, records]
  )

  const handleDragOver = useCallback((event: DragEvent) => {
    event.preventDefault()
    event.dataTransfer.dropEffect = 'move'
  }, [])

  const handleNodesChange = useCallback(
    (changes: NodeChange<WorkflowRecordNode>[]) => {
      hasLocalCanvasChanges.current = true
      localNodeDragActive.current = changes.some(
        (change) =>
          change.type === 'position' &&
          'dragging' in change &&
          change.dragging === true
      )
      setNodes((current) => applyNodeChanges(changes, current))
    },
    []
  )

  const handleEdgesChange = useCallback((changes: EdgeChange<Edge>[]) => {
    if (changes.some((change) => change.type !== 'select')) {
      hasLocalCanvasChanges.current = true
    }
    if (
      selectedEdgeId &&
      changes.some((change) => change.type === 'remove' && change.id === selectedEdgeId)
    ) {
      setSelectedEdgeId(null)
    }
    setEdges((current) => applyEdgeChanges(changes, current))
  }, [selectedEdgeId])

  const handleConnect = useCallback((connection: Connection) => {
    hasLocalCanvasChanges.current = true
    setEdges((current) =>
      addEdge(
        {
          ...connection,
          type: 'smoothstep',
          animated: true,
          style: { stroke: 'var(--accent)' },
        },
        current
      )
    )
  }, [])

  const handleNodeClick = useCallback(
    (_event: MouseEvent, node: WorkflowRecordNode) => {
      setPreviewRecordId(node.data.recordId)
    },
    []
  )

  const handleSelectionChange = useCallback(
    ({ nodes: selectedNodes, edges: selectedEdges }: {
      nodes: WorkflowRecordNode[]
      edges: Edge[]
    }) => {
      if (selectedNodes.length === 1) {
        setPreviewRecordId(selectedNodes[0].data.recordId)
      }
      setSelectedEdgeId(selectedEdges.length === 1 ? selectedEdges[0].id : null)
    },
    []
  )

  const deleteSelectedEdge = useCallback(() => {
    if (!selectedEdgeId) return
    hasLocalCanvasChanges.current = true
    setEdges((current) => current.filter((edge) => edge.id !== selectedEdgeId))
    setSelectedEdgeId(null)
  }, [selectedEdgeId])

  return (
    <div className="relative flex h-full min-h-0 bg-canvas">
      {!itemsPanelOpen && (
        <aside className="hidden w-16 shrink-0 border-r border-border bg-panel p-2 md:flex md:flex-col md:items-center">
          <button
            type="button"
            data-testid="toggle-workflow-items"
            className="flex min-h-7 w-full items-center justify-center rounded bg-surface-hover px-1.5 py-1 text-[10px] font-medium text-muted hover:text-foreground"
            title="Open Library data"
            onClick={() => setItemsPanelOpen(true)}
          >
            Items
          </button>
        </aside>
      )}

      {itemsPanelOpen && (
      <aside
        data-testid="workflow-elements-panel"
        className="absolute inset-x-2 bottom-2 top-2 z-30 flex min-h-0 shrink-0 flex-col rounded-md border border-border bg-panel p-2.5 shadow-soft md:static md:inset-auto md:z-auto md:w-72 md:rounded-none md:border-y-0 md:border-l-0 md:shadow-none"
      >
        <div className="mb-2 px-0.5">
          <div className="flex items-center justify-between gap-2">
            <div className="text-xs font-medium uppercase tracking-wider text-subtle">
              Library Data
            </div>
            <button
              type="button"
              data-testid="toggle-workflow-items"
              className="flex h-7 w-7 items-center justify-center rounded bg-surface-hover text-muted hover:text-foreground"
              title="Close Library data"
              aria-label="Close Library data"
              onClick={() => setItemsPanelOpen(false)}
            >
              <PanelLeftClose className="h-3.5 w-3.5" aria-hidden="true" />
            </button>
          </div>
          <p className="mt-1 text-[11px] leading-snug text-subtle">
            Add data to the canvas.
          </p>
        </div>
        <div className="mb-2 px-0.5">
          <div className="relative">
            <Search
              className="pointer-events-none absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-subtle"
              aria-hidden="true"
            />
            <input
              type="search"
              aria-label="Search Library data"
              value={recordQuery}
              onChange={(event) => {
                setRecordQuery(event.target.value)
                setRecordPage(0)
              }}
              placeholder="Search items"
              className="h-8 w-full rounded border border-border bg-surface pl-7 pr-2 text-xs text-foreground outline-none placeholder:text-subtle focus:border-accent"
            />
          </div>
          <p className="mt-1 text-[10px] tabular-nums text-subtle" data-testid="workflow-item-count">
            Showing {visibleRecordStart}-{visibleRecordEnd} of {matchingRecords.length} matching items
          </p>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto pr-1">
          <div className="flex flex-col gap-1">
            {visibleRecords.map((record) => (
              <div
                key={record.id}
                draggable
                onDragStart={(event) => handleDragStart(event, record)}
                className="group flex items-center gap-2 rounded border border-border bg-surface px-2 py-1.5"
                data-testid="workflow-database-item"
              >
                <span
                  className="h-2 w-2 shrink-0 rounded-full"
                  style={{ background: statusConfig[record.status].color }}
                />
                <div className="min-w-0 flex-1">
                  <div className="flex min-w-0 items-center gap-1.5">
                    <span className="shrink-0 text-[10px] font-medium text-subtle">
                      {record.identifier}
                    </span>
                    <span className="truncate text-xs font-medium text-foreground">
                      {record.title}
                    </span>
                  </div>
                  <div className="mt-0.5 flex items-center gap-1.5 text-[10px] text-subtle">
                    <span>{statusConfig[record.status].label}</span>
                    <span style={{ color: priorityConfig[record.priority].color }}>
                      {priorityConfig[record.priority].icon} {priorityConfig[record.priority].label}
                    </span>
                  </div>
                </div>
                <button
                  type="button"
                  data-testid="workflow-add-record"
                  className="shrink-0 rounded bg-surface-hover px-1.5 py-0.5 text-[10px] font-medium text-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                  disabled={!workflowLoaded}
                  onClick={() => addRecord(record)}
                >
                  Add
                </button>
              </div>
            ))}
            {matchingRecords.length === 0 && (
              <div className="rounded-md border border-dashed border-border bg-surface p-3 text-xs leading-relaxed text-subtle">
                {records.length === 0
                  ? 'No items in this database yet.'
                  : 'No items match this search.'}
              </div>
            )}
            {recordPageCount > 1 && (
              <div className="sticky bottom-0 mt-1 flex items-center justify-between gap-2 rounded border border-border bg-panel px-2 py-1.5">
                <button
                  type="button"
                  className="rounded bg-surface-hover px-2 py-1 text-[10px] font-medium text-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                  disabled={visibleRecordPage === 0}
                  aria-label="Previous Library data page"
                  onClick={() => setRecordPage((page) => Math.max(0, page - 1))}
                >
                  Previous
                </button>
                <span className="text-[10px] tabular-nums text-subtle">
                  {visibleRecordPage + 1}/{recordPageCount}
                </span>
                <button
                  type="button"
                  className="rounded bg-surface-hover px-2 py-1 text-[10px] font-medium text-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                  disabled={visibleRecordPage >= recordPageCount - 1}
                  aria-label="Next Library data page"
                  onClick={() => setRecordPage((page) => Math.min(recordPageCount - 1, page + 1))}
                >
                  Next
                </button>
              </div>
            )}
          </div>
        </div>
      </aside>
      )}

      <div className="flex min-w-0 flex-1 flex-col p-1 md:p-2">
        <div className="shrink-0 border-b border-border px-3 py-2 md:px-4">
          <div className="flex flex-wrap items-center gap-2">
            {!itemsPanelOpen && (
              <button
                type="button"
                className="inline-flex items-center gap-1 rounded bg-surface-hover px-2 py-1 text-[11px] font-medium text-muted hover:text-foreground md:hidden"
                aria-label="Open Library data"
                onClick={() => setItemsPanelOpen(true)}
              >
                <ListPlus className="h-3.5 w-3.5" aria-hidden="true" />
                Items
              </button>
            )}
            <span className="text-xs font-medium text-foreground">Workflow Canvas</span>
            <span className="min-w-0 truncate text-xs text-subtle">
              {selectedTemplate.canvasHint}
            </span>
            <span
              className="ml-auto text-[10px] font-medium uppercase tracking-wider text-subtle"
              data-saved-signature={savedCountSignature}
              data-testid="workflow-persistence-status"
            >
              {saveStatus === 'saving' || savedSignature !== canvasSignature
                ? 'Saving'
                : 'Saved'}
            </span>
          </div>
          <div className="mt-2 flex flex-wrap gap-1">
            {workflowTemplates.map((template) => (
              <button
                type="button"
                key={template.id}
                data-testid={`workflow-template-${template.id}`}
                className={`rounded px-2.5 py-1.5 text-xs font-medium transition-colors ${
                  selectedTemplateId === template.id
                    ? 'bg-accent text-white'
                    : 'bg-surface-hover text-muted hover:text-foreground'
                }`}
                onClick={() => setSelectedTemplateId(template.id)}
              >
                {template.label}
              </button>
            ))}
            <span className="ml-1 self-center text-xs text-subtle">
              {selectedTemplate.description}
            </span>
            {selectedEdgeId && (
              <button
                type="button"
                data-testid="workflow-delete-edge"
                className="ml-auto inline-flex items-center gap-1 rounded bg-status-cancelled px-2.5 py-1.5 text-xs font-medium text-white"
                onClick={deleteSelectedEdge}
              >
                <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
                Delete connection
              </button>
            )}
          </div>
        </div>

        <div
          ref={wrapperRef}
          className="relative min-h-0 flex-1 overflow-hidden rounded-md bg-canvas"
          data-testid="workflow-canvas"
          onDrop={handleDrop}
          onDragOver={handleDragOver}
        >
          <ReactFlow<WorkflowRecordNode>
            nodes={nodes}
            edges={renderedEdges}
            nodeTypes={nodeTypes}
            nodesDraggable
            nodesConnectable
            connectOnClick
            edgesFocusable
            onNodesChange={handleNodesChange}
            onEdgesChange={handleEdgesChange}
            onConnect={handleConnect}
            onNodeClick={handleNodeClick}
            onEdgeClick={(_event, edge) => setSelectedEdgeId(edge.id)}
            onSelectionChange={handleSelectionChange}
            onPaneClick={() => setSelectedEdgeId(null)}
            onInit={setReactFlowInstance}
            fitView
            fitViewOptions={{ padding: 0.25 }}
            minZoom={0.35}
            maxZoom={1.6}
          >
            <Background
              variant={BackgroundVariant.Dots}
              gap={24}
              size={1}
              color="var(--border-color)"
            />
            <MiniMap
              nodeColor={(node) => (node as WorkflowRecordNode).data.accent}
              maskColor="rgba(10, 10, 15, 0.55)"
              pannable
              zoomable
            />
            <Controls showInteractive={false} />
          </ReactFlow>
        </div>
      </div>
      {previewRecord && (
        <DetailPanel
          record={previewRecord}
          onClose={() => setPreviewRecordId(null)}
          onUpdateRecord={onUpdateRecord}
          onDeleteRecord={
            onDeleteRecord
              ? (recordId: string) => {
                  onDeleteRecord(recordId)
                  setPreviewRecordId(null)
                }
              : undefined
          }
        />
      )}
    </div>
  )
}
