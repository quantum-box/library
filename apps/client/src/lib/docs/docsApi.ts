import { appKitConfig } from '../../app/kitConfig'
import type { CreateDocInput, DocMetadata, UpdateDocInput } from './types'
import {
  deleteClientEngineRecord,
  listClientEngineRecords,
  getClientEngineRecord,
  patchClientEngineRecord,
  syncClientEngineOperations,
  upsertClientEngineRecord,
} from '../photonEngine/client'

export interface ServerDocumentMetadata {
  id: string
  title: string
  workspace_id: string
  created_at: string
  updated_at: string
}

export interface ServerDocumentListResponse {
  documents: ServerDocumentMetadata[]
  total: number
}

export class DocsApiError extends Error {
  readonly status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'DocsApiError'
    this.status = status
  }
}

const DOCUMENT_SYNC_TIMEOUT_MS = 5_000

function withoutUndefined<T extends object>(payload: T): Partial<T> {
  return Object.fromEntries(
    Object.entries(payload).filter(([, value]) => value !== undefined)
  ) as Partial<T>
}

export function toDocMetadata(serverDocument: ServerDocumentMetadata): DocMetadata {
  return {
    id: serverDocument.id,
    title: serverDocument.title || appKitConfig.docs.defaultTitle,
    workspaceId: serverDocument.workspace_id || appKitConfig.workspace.id,
    createdAt: serverDocument.created_at,
    updatedAt: serverDocument.updated_at,
  }
}

async function syncDocumentsBestEffort(): Promise<boolean> {
  const controller = new AbortController()
  const timeout = globalThis.setTimeout(() => controller.abort(), DOCUMENT_SYNC_TIMEOUT_MS)
  try {
    await syncClientEngineOperations(undefined, controller.signal)
    return true
  } catch (error: unknown) {
    console.warn('Failed to sync document metadata; using the local Photon Engine projection', error)
    return false
  } finally {
    globalThis.clearTimeout(timeout)
  }
}

export async function fetchServerDocuments(): Promise<DocMetadata[]> {
  await syncDocumentsBestEffort()
  const records = await listClientEngineRecords<DocMetadata>('documents')
  return records
    .map((record) => record.value)
    .filter((document) => document.workspaceId === appKitConfig.workspace.id)
}

export async function fetchServerDocument(docId: string): Promise<DocMetadata> {
  const syncSucceeded = await syncDocumentsBestEffort()
  const record = await getClientEngineRecord<DocMetadata>('documents', docId)
  if (!record && syncSucceeded) throw new DocsApiError('Document metadata not found', 404)
  if (!record) throw new Error('Document metadata is temporarily unavailable')
  return record.value
}

export async function createServerDocument(input: CreateDocInput = {}): Promise<DocMetadata> {
  const now = new Date().toISOString()
  const document: DocMetadata = {
    id: input.id ?? globalThis.crypto?.randomUUID?.() ?? `doc-${Date.now()}`,
    title: input.title?.trim() || appKitConfig.docs.defaultTitle,
    workspaceId: appKitConfig.workspace.id,
    createdAt: now,
    updatedAt: now,
  }
  const record = await upsertClientEngineRecord('documents', document.id, document)
  await syncDocumentsBestEffort()
  return record.value
}

export async function updateServerDocument(
  docId: string,
  input: UpdateDocInput
): Promise<DocMetadata> {
  const existing = await fetchServerDocument(docId)
  const document: DocMetadata = {
    ...existing,
    ...withoutUndefined({ title: input.title?.trim() || appKitConfig.docs.defaultTitle }),
    updatedAt: new Date().toISOString(),
  }
  const record = await patchClientEngineRecord<DocMetadata>('documents', docId, document)
  if (!record) throw new DocsApiError('Document metadata not found', 404)
  await syncDocumentsBestEffort()
  return record.value
}

export async function deleteServerDocument(docId: string): Promise<void> {
  await fetchServerDocument(docId)
  await deleteClientEngineRecord('documents', docId)
  await syncDocumentsBestEffort()
}
