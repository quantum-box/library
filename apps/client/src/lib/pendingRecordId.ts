const PREFIX = 'optimistic-record-'

export function createPendingRecordId() {
  const randomId = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`
  return `${PREFIX}${randomId}`
}

export function isPendingRecordId(id: string) { return id.startsWith(PREFIX) }
