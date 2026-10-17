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
import { hiddenWorkflowNodes, layoutWorkflowNodes, normalizeWorkflowSettings, wouldCreateCycle, type WorkflowSettings } from '../lib/workflows/workflowGraph'
import { getLibraryDataPropertyValue, type LibraryDataItem, type LibraryProperty } from '../lib/recordsApi'
import { propertyValueDisplayText } from '../lib/libraryTable/libraryPropertyFormat'
import { numberFormatter } from '../i18n/format'
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
import { getIsMobileViewport } from '../lib/ui/useIsMobileViewport'
import { DetailPanel } from './DetailPanel'
import { useI18n, type MessageKey } from '../i18n'

type WorkflowTemplateId = WorkflowCanvasTemplateId

type WorkflowTemplate = {
  id: WorkflowTemplateId
  labelKey: MessageKey
  descriptionKey: MessageKey
  canvasHintKey: MessageKey
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
  fields?: { id: string; label: string; value: string }[]
  collapsed?: boolean
  hasChildren?: boolean
  onToggle?: () => void
}

type WorkflowRecordNode = Node<WorkflowNodeData, 'workflowRecord'>

const workflowTemplates: WorkflowTemplate[] = [
  {
    id: 'business-flow',
    labelKey: 'workflow.template.graph.label',
    descriptionKey: 'workflow.template.graph.description',
    canvasHintKey: 'workflow.template.graph.canvasHint',
  },
  {
    id: 'tree',
    labelKey: 'workflow.template.tree.label',
    descriptionKey: 'workflow.template.tree.description',
    canvasHintKey: 'workflow.template.tree.canvasHint',
  },
]

function WorkflowRecordNode({ data, selected }: NodeProps<WorkflowRecordNode>) {
  const { t } = useI18n()

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
          {t(priorityConfig[data.priority].labelKey)}
        </span>
      </div>
      <div className="line-clamp-2 text-xs leading-relaxed text-subtle">
        {data.description}
      </div>
      {data.fields?.map((field) => <div key={field.id} className="mt-2 flex justify-between gap-3 text-xs">
        <span className="truncate text-subtle">{field.label}</span>
        <span className="max-w-36 truncate text-foreground" title={field.value}>{field.value}</span>
      </div>)}
      {data.hasChildren && <button type="button" className="nodrag nopan mt-2 rounded bg-surface-hover px-2 py-1 text-xs"
        aria-expanded={!data.collapsed} onClick={(event) => { event.stopPropagation(); data.onToggle?.() }}>
        {t(data.collapsed ? 'workflow.expand' : 'workflow.collapse')}
      </button>}
      <div className="mt-3 flex items-center justify-between gap-2 text-[10px] text-subtle">
        <span style={{ color: statusConfig[data.status].color }}>
          {statusConfig[data.status].icon} {t(statusConfig[data.status].labelKey)}
        </span>
        <span>
          {data.templateId === 'tree'
            ? t('workflow.node.treeItem')
            : t('workflow.node.flowItem')}
        </span>
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
  settings: WorkflowSettings
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
  return value === 'business-flow' || value === 'tree'
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
  properties = [], items = [], repositoryId, onChangeRelation, onCreateRecord, onOpenRecord,
}: {
  databaseId: string
  properties?: LibraryProperty[]
  items?: LibraryDataItem[]
  repositoryId?: string
  onChangeRelation?: (propertyId: string, source: string, target: string, remove: boolean) => Promise<void>
  onCreateRecord?: (name: string, parentId?: string, relationPropertyId?: string) => Promise<DatabaseRecord>
  onOpenRecord?: (record: DatabaseRecord) => void
  records: DatabaseRecord[]
  onUpdateRecord?: (recordId: string, field: keyof DatabaseRecord, value: string) => void
  onDeleteRecord?: (recordId: string) => void
}) {
  const { t, locale } = useI18n()
  const [settings, setSettings] = useState<WorkflowSettings>(() => normalizeWorkflowSettings())
  const [operationError, setOperationError] = useState('')
  const [busy, setBusy] = useState(false)
  const [newTitle, setNewTitle] = useState('')
  const [parentNodeId, setParentNodeId] = useState('')
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)
  const wrapperRef = useRef<HTMLDivElement | null>(null)
  const [selectedTemplateId, setSelectedTemplateId] =
    useState<WorkflowTemplateId>('business-flow')
  // The picker is a full-screen overlay on a phone, so opening it by default
  // would hide the canvas the view exists to show. Desktop docks it beside the
  // canvas and keeps it open.
  const [itemsPanelOpen, setItemsPanelOpen] = useState(() => !getIsMobileViewport())
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
      t(statusConfig[record.status].labelKey),
      t(priorityConfig[record.priority].labelKey),
    ].some((value) => value.toLowerCase().includes(normalizedRecordQuery)))
  }, [normalizedRecordQuery, records, t])
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
  const changeSettings = useCallback((patch: Partial<WorkflowSettings>) => {
    hasLocalCanvasChanges.current = true
    setSettings((current) => ({ ...current, ...patch }))
  }, [])
  const relationProperties = properties.filter((property) => property.typ === 'Relation' && repositoryId && property.meta?.databaseId === repositoryId)
  const relationProperty = relationProperties.find((property) => property.id === settings.relationPropertyId)
  const relationEdges = useMemo<Edge[]>(() => {
    if (!relationProperty) return []
    return nodes.flatMap((source) => {
      const item = items.find((candidate) => candidate.id === source.data.recordId)
      const ids = item && getLibraryDataPropertyValue(item, relationProperty.id)?.dataIds
      return nodes.filter((target) => ids?.includes(target.data.recordId)).map((target) => ({
        id: `relation:${relationProperty.id}:${source.id}:${target.id}`,
        source: source.id, target: target.id, type: 'smoothstep',
        label: relationProperty.displayName || relationProperty.name,
        data: { relation: true, sourceRecordId: source.data.recordId, targetRecordId: target.data.recordId },
        style: { stroke: 'var(--accent)' },
      }))
    })
  }, [items, nodes, relationProperty])
  const allEdges = useMemo(() => [...edges, ...relationEdges], [edges, relationEdges])
  const hidden = useMemo(() => hiddenWorkflowNodes(allEdges, settings.collapsedNodeIds), [allEdges, settings.collapsedNodeIds])
  const renderedEdges = useMemo(() => allEdges.map((edge) => ({
    ...edge, selected: edge.id === selectedEdgeId,
    hidden: hidden.has(edge.source) || hidden.has(edge.target),
  })), [allEdges, hidden, selectedEdgeId])
  const renderedNodes = useMemo(() => nodes.map((node) => {
    const item = items.find((candidate) => candidate.id === node.data.recordId)
    const record = records.find((candidate) => candidate.id === node.data.recordId)
    return { ...node, hidden: hidden.has(node.id), data: {
      ...node.data, ...(record ? { title: record.title, description: record.description, status: record.status, priority: record.priority } : {}),
      fields: properties.filter((property) => settings.visiblePropertyIds.includes(property.id)).map((property) => {
        const value = item && getLibraryDataPropertyValue(item, property.id)
        let text = value ? propertyValueDisplayText(property, value) ?? '' : ''
        if (value?.number && Number.isFinite(Number(value.number))) text = numberFormatter(locale).format(Number(value.number))
        if (value?.dataIds) text = value.dataIds.map((id) => items.find((candidate) => candidate.id === id)?.name ?? id).join(', ')
        return { id: property.id, label: property.displayName || property.name, value: text || '—' }
      }),
      collapsed: settings.collapsedNodeIds.includes(node.id),
      hasChildren: allEdges.some((edge) => edge.source === node.id),
      onToggle: () => changeSettings({ collapsedNodeIds: settings.collapsedNodeIds.includes(node.id)
        ? settings.collapsedNodeIds.filter((id) => id !== node.id) : [...settings.collapsedNodeIds, node.id] }),
    } }
  }), [nodes, items, records, properties, settings, hidden, allEdges, locale, changeSettings])
  const canvasSignature = useMemo(
    () =>
      JSON.stringify({
        selectedTemplateId,
        settings,
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
          label: edge.label,
        })),
      }),
    [edges, nodes, selectedTemplateId, settings]
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
        settings: payload.settings,
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
      setSettings(normalizeWorkflowSettings(canvas.settings))
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
    setSettings(normalizeWorkflowSettings())
    setOperationError('')
    setNewTitle('')
    setParentNodeId('')
    setSelectedNodeId(null)
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

    // Initial hydration is handled above. Re-subscribing after a record changes
    // must not replace unsaved local placement with the previous canvas.
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
        settings,
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
        settings,
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
    settings,
  ])

  useEffect(() => {
    window.addEventListener('pagehide', flushWorkflowSync)
    return () => {
      if (syncThrottleTimer.current !== null) {
        window.clearTimeout(syncThrottleTimer.current)
      }
      flushWorkflowSync()
      window.removeEventListener('pagehide', flushWorkflowSync)
    }
  }, [flushWorkflowSync])

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

  const runOperation = async (action: () => Promise<void>) => {
    if (busy) return
    setBusy(true)
    setOperationError('')
    try { await action() } catch (error) {
      setOperationError(error instanceof Error && error.message === t('workflow.createdUnlinked')
        ? error.message : t('workflow.operationFailed'))
    } finally { setBusy(false) }
  }
  const handleConnect = (connection: Connection) => {
    if (busy || !connection.source || !connection.target) return
    const cyclic = wouldCreateCycle(allEdges, connection.source, connection.target)
    if (connection.source === connection.target || (selectedTemplateId === 'tree' && cyclic)) {
      setOperationError(t('workflow.cycleRejected'))
      return
    }
    if (relationProperty && onChangeRelation) {
      const source = nodes.find((node) => node.id === connection.source)
      const target = nodes.find((node) => node.id === connection.target)
      if (source && target) void runOperation(() => onChangeRelation(relationProperty.id, source.data.recordId, target.data.recordId, false))
    } else {
      hasLocalCanvasChanges.current = true
      setEdges((current) => addEdge({ ...connection, type: 'smoothstep', style: { stroke: 'var(--accent)' } }, current))
    }
  }
  const arrange = () => {
    const positions = layoutWorkflowNodes(nodes, allEdges, 220 + settings.visiblePropertyIds.length * 28)
    hasLocalCanvasChanges.current = true
    setNodes((current) => current.map((node) => ({ ...node, position: positions.get(node.id)! })))
    requestAnimationFrame(() => { void reactFlowInstance?.fitView({ padding: 0.25 }) })
  }
  const createNode = () => {
    if (!onCreateRecord || !newTitle.trim()) return
    const parent = nodes.find((node) => node.id === parentNodeId)
    void runOperation(async () => {
      const record = await onCreateRecord(newTitle.trim(), parent?.data.recordId, relationProperty?.id)
      nodeSequence.current += 1
      const node = createRecordNode(record, selectedTemplateId, parent
        ? { x: parent.position.x + 340, y: parent.position.y + 220 } : { x: 80, y: 80 }, nodeSequence.current)
      hasLocalCanvasChanges.current = true
      setNodes((current) => [...current, node])
      if (parent && !relationProperty) setEdges((current) => addEdge({ id: `annotation:${parent.id}:${node.id}`, source: parent.id, target: node.id, type: 'smoothstep' }, current))
      setNewTitle('')
    })
  }

  const handleNodeClick = useCallback(
    (_event: MouseEvent, node: WorkflowRecordNode) => {
      setSelectedNodeId(node.id)
      if (!onOpenRecord) setPreviewRecordId(node.data.recordId)
    },
    [onOpenRecord]
  )

  const handleSelectionChange = useCallback(
    ({ nodes: selectedNodes, edges: selectedEdges }: {
      nodes: WorkflowRecordNode[]
      edges: Edge[]
    }) => {
      setSelectedNodeId(selectedNodes.length === 1 ? selectedNodes[0].id : null)
      if (selectedNodes.length === 1 && !onOpenRecord) {
        setPreviewRecordId(selectedNodes[0].data.recordId)
      }
      setSelectedEdgeId(selectedEdges.length === 1 ? selectedEdges[0].id : null)
    },
    [onOpenRecord]
  )

  const deleteSelectedEdge = () => {
    if (!selectedEdgeId || busy) return
    const relation = relationEdges.find((edge) => edge.id === selectedEdgeId)
    if (relation && onChangeRelation && relationProperty) {
      void runOperation(async () => {
        await onChangeRelation(relationProperty.id, relation.data!.sourceRecordId as string, relation.data!.targetRecordId as string, true)
        setSelectedEdgeId(null)
      })
      return
    }
    hasLocalCanvasChanges.current = true
    setEdges((current) => current.filter((edge) => edge.id !== selectedEdgeId))
    setSelectedEdgeId(null)
  }

  return (
    <div className="relative flex h-full min-h-0 bg-canvas">
      {!itemsPanelOpen && (
        <aside className="hidden w-16 shrink-0 border-r border-border bg-panel p-2 md:flex md:flex-col md:items-center">
          <button
            type="button"
            data-testid="toggle-workflow-items"
            className="flex min-h-7 w-full items-center justify-center rounded bg-surface-hover px-1.5 py-1 text-[10px] font-medium text-muted hover:text-foreground"
            title={t('workflow.openLibraryData')}
            onClick={() => setItemsPanelOpen(true)}
          >
            {t('workflow.items')}
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
              {t('workflow.libraryData')}
            </div>
            <button
              type="button"
              data-testid="toggle-workflow-items"
              className="flex h-7 w-7 items-center justify-center rounded bg-surface-hover text-muted hover:text-foreground"
              title={t('workflow.closeLibraryData')}
              aria-label={t('workflow.closeLibraryData')}
              onClick={() => setItemsPanelOpen(false)}
            >
              <PanelLeftClose className="h-3.5 w-3.5" aria-hidden="true" />
            </button>
          </div>
          <p className="mt-1 text-[11px] leading-snug text-subtle">
            {t('workflow.addDataHint')}
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
              aria-label={t('workflow.searchLibraryData')}
              value={recordQuery}
              onChange={(event) => {
                setRecordQuery(event.target.value)
                setRecordPage(0)
              }}
              placeholder={t('workflow.searchItems')}
              className="h-8 w-full rounded border border-border bg-surface pl-7 pr-2 text-xs text-foreground outline-none placeholder:text-subtle focus:border-accent"
            />
          </div>
          <p className="mt-1 text-[10px] tabular-nums text-subtle" data-testid="workflow-item-count">
            {t('workflow.showingRange', {
              from: visibleRecordStart,
              to: visibleRecordEnd,
              total: matchingRecords.length,
            })}
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
                    <span>{t(statusConfig[record.status].labelKey)}</span>
                    <span style={{ color: priorityConfig[record.priority].color }}>
                      {priorityConfig[record.priority].icon}{' '}
                      {t(priorityConfig[record.priority].labelKey)}
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
                  {t('common.add')}
                </button>
              </div>
            ))}
            {matchingRecords.length === 0 && (
              <div className="rounded-md border border-dashed border-border bg-surface p-3 text-xs leading-relaxed text-subtle">
                {records.length === 0
                  ? t('workflow.noItems')
                  : t('workflow.noMatchingItems')}
              </div>
            )}
            {recordPageCount > 1 && (
              <div className="sticky bottom-0 mt-1 flex items-center justify-between gap-2 rounded border border-border bg-panel px-2 py-1.5">
                <button
                  type="button"
                  className="rounded bg-surface-hover px-2 py-1 text-[10px] font-medium text-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                  disabled={visibleRecordPage === 0}
                  aria-label={t('workflow.previousPage')}
                  onClick={() => setRecordPage((page) => Math.max(0, page - 1))}
                >
                  {t('common.previous')}
                </button>
                <span className="text-[10px] tabular-nums text-subtle">
                  {visibleRecordPage + 1}/{recordPageCount}
                </span>
                <button
                  type="button"
                  className="rounded bg-surface-hover px-2 py-1 text-[10px] font-medium text-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                  disabled={visibleRecordPage >= recordPageCount - 1}
                  aria-label={t('workflow.nextPage')}
                  onClick={() => setRecordPage((page) => Math.min(recordPageCount - 1, page + 1))}
                >
                  {t('common.next')}
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
                aria-label={t('workflow.openLibraryData')}
                onClick={() => setItemsPanelOpen(true)}
              >
                <ListPlus className="h-3.5 w-3.5" aria-hidden="true" />
                {t('workflow.items')}
              </button>
            )}
            <span className="text-xs font-medium text-foreground">{t('workflow.canvas')}</span>
            <span className="min-w-0 truncate text-xs text-subtle">
              {t(selectedTemplate.canvasHintKey)}
            </span>
            <span
              className="ml-auto text-[10px] font-medium uppercase tracking-wider text-subtle"
              data-saved-signature={savedCountSignature}
              data-testid="workflow-persistence-status"
            >
              {saveStatus === 'saving' || savedSignature !== canvasSignature
                ? t('workflow.saving')
                : t('common.saved')}
            </span>
          </div>
          <div className="mt-2 flex flex-wrap gap-1">
            {workflowTemplates.map((template) => (
              <button
                type="button"
                key={template.id}
                disabled={!workflowLoaded}
                data-testid={`workflow-template-${template.id}`}
                className={`rounded px-2.5 py-1.5 text-xs font-medium transition-colors ${
                  selectedTemplateId === template.id
                    ? 'bg-accent text-white'
                    : 'bg-surface-hover text-muted hover:text-foreground'
                }`}
                onClick={() => setSelectedTemplateId(template.id)}
              >
                {t(template.labelKey)}
              </button>
            ))}
            <span className="ml-1 self-center text-xs text-subtle">
              {t(selectedTemplate.descriptionKey)}
            </span>
            {selectedEdgeId && (
              <button
                type="button"
                data-testid="workflow-delete-edge"
                className="ml-auto inline-flex items-center gap-1 rounded bg-status-cancelled px-2.5 py-1.5 text-xs font-medium text-white"
                onClick={deleteSelectedEdge}
              >
                <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
                {t('workflow.deleteConnection')}
              </button>
            )}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2 border-b border-border px-3 py-2 text-xs">
          <button type="button" disabled={!workflowLoaded} onClick={arrange} className="rounded bg-surface-hover px-2 py-1">{t('workflow.arrange')}</button>
          <button type="button" onClick={() => changeSettings({ collapsedNodeIds: [] })} className="rounded bg-surface-hover px-2 py-1">{t('workflow.expandAll')}</button>
          {selectedNodeId && <>
            <button type="button" disabled={busy} className="rounded bg-surface-hover px-2 py-1" onClick={() => {
              hasLocalCanvasChanges.current = true
              setNodes((current) => current.filter((node) => node.id !== selectedNodeId))
              setEdges((current) => current.filter((edge) => edge.source !== selectedNodeId && edge.target !== selectedNodeId))
              setSelectedNodeId(null)
            }}>{t('workflow.removeFromView')}</button>
            {onOpenRecord && <button type="button" className="rounded bg-surface-hover px-2 py-1" onClick={() => {
              const node = nodes.find((candidate) => candidate.id === selectedNodeId)
              const record = records.find((candidate) => candidate.id === node?.data.recordId)
              if (record) onOpenRecord(record)
            }}>{t('workflow.openData')}</button>}
          </>}
          {properties.length > 0 && <details className="relative">
            <summary className="cursor-pointer rounded bg-surface-hover px-2 py-1">{t('workflow.properties')}</summary>
            <div className="absolute left-0 top-full z-30 mt-1 max-h-64 w-56 overflow-auto rounded border border-border bg-panel p-2 shadow-soft">
              {properties.map((property) => <label key={property.id} className="flex items-center gap-2 p-1">
                <input type="checkbox" checked={settings.visiblePropertyIds.includes(property.id)} onChange={(event) => changeSettings({ visiblePropertyIds: event.target.checked ? [...settings.visiblePropertyIds, property.id] : settings.visiblePropertyIds.filter((id) => id !== property.id) })} />
                {property.displayName || property.name}
              </label>)}
            </div>
          </details>}
          {relationProperties.length > 0 && <label className="flex items-center gap-2">{t('workflow.connections')}
            <select aria-label={t('workflow.connections')} disabled={busy} className="rounded border border-border bg-panel p-1" value={relationProperty?.id ?? ''}
              onChange={(event) => { setSelectedEdgeId(null); changeSettings({ relationPropertyId: event.target.value }) }}>
              <option value="">{t('workflow.annotations')}</option>
              {relationProperties.map((property) => <option key={property.id} value={property.id}>{property.displayName || property.name}</option>)}
            </select>
          </label>}
          {onCreateRecord && <form className="flex flex-wrap items-center gap-2" onSubmit={(event) => { event.preventDefault(); createNode() }}>
            <input className="rounded border border-border bg-panel p-1" aria-label={t('workflow.newData')} placeholder={t('workflow.newData')} value={newTitle} onChange={(event) => setNewTitle(event.target.value)} disabled={busy} />
            <select className="max-w-40 rounded border border-border bg-panel p-1" aria-label={t('workflow.parent')} value={parentNodeId} onChange={(event) => setParentNodeId(event.target.value)} disabled={busy}>
              <option value="">{t('workflow.noParent')}</option>
              {nodes.map((node) => <option key={node.id} value={node.id}>{node.data.title}</option>)}
            </select>
            <button type="submit" disabled={busy || !newTitle.trim() || !workflowLoaded} className="rounded bg-surface-hover px-2 py-1 disabled:opacity-40">{t('common.add')}</button>
          </form>}
          {selectedEdgeId && !relationEdges.some((edge) => edge.id === selectedEdgeId) && <label className="flex items-center gap-2">{t('workflow.edgeLabel')}
            <input aria-label={t('workflow.edgeLabel')} className="rounded border border-border bg-panel p-1" value={String(edges.find((edge) => edge.id === selectedEdgeId)?.label ?? '')}
              onChange={(event) => { hasLocalCanvasChanges.current = true; setEdges((current) => current.map((edge) => edge.id === selectedEdgeId ? { ...edge, label: event.target.value } : edge)) }} />
          </label>}
          {relationProperty && <span className="text-subtle">{t('workflow.relationHint')}</span>}
          {operationError && <div role="alert" className="w-full text-red-500">{operationError}</div>}
        </div>

        <div
          ref={wrapperRef}
          className="relative min-h-0 flex-1 overflow-hidden rounded-md bg-canvas"
          data-testid="workflow-canvas"
          onDrop={handleDrop}
          onDragOver={handleDragOver}
        >
          <ReactFlow<WorkflowRecordNode>
            nodes={renderedNodes}
            edges={renderedEdges}
            nodeTypes={nodeTypes}
            nodesDraggable
            nodesConnectable
            connectOnClick
            edgesFocusable
            onNodesChange={handleNodesChange}
            onEdgesChange={(changes) => handleEdgesChange(changes.filter((change) => !('id' in change) || !String(change.id).startsWith('relation:')))}
            deleteKeyCode={null}
            onConnect={handleConnect}
            onNodeClick={handleNodeClick}
            onNodeDoubleClick={(_event, node) => {
              const record = records.find((candidate) => candidate.id === node.data.recordId)
              if (record) onOpenRecord?.(record)
            }}
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
