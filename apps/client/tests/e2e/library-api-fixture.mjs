import { createServer } from 'node:http'
import { randomUUID } from 'node:crypto'
import { WebSocket, WebSocketServer } from 'ws'
import * as Y from 'yjs'

const host = '127.0.0.1'
const port = Number(process.env.LIBRARY_E2E_API_PORT ?? 50063)
const platformId = 'tn_01j702qf86pc2j35s0kv0gv3gy'

const repositoryTemplate = {
  id: 'repo-1',
  username: 'photon-core',
  name: 'Photon Core',
  description: 'Library E2E repository',
  isPublic: false,
  policies: [{ userId: 'library-e2e-user', role: 'owner' }],
}

const option = (prefix, key, name) => ({ id: `${prefix}-${key}`, key, name })

const propertyTemplates = [
  { id: 'prop-identifier', name: 'Identifier', typ: 'Id', meta: { autoGenerate: true } },
  {
    id: 'prop-status',
    name: 'Status',
    typ: 'Select',
    meta: {
      options: [
        option('status', 'backlog', 'Backlog'),
        option('status', 'todo', 'Todo'),
        option('status', 'in_progress', 'In Progress'),
        option('status', 'in_review', 'In Review'),
        option('status', 'done', 'Done'),
        option('status', 'cancelled', 'Cancelled'),
      ],
    },
  },
  {
    id: 'prop-priority',
    name: 'Priority',
    typ: 'Select',
    meta: {
      options: [
        option('priority', 'urgent', 'Urgent'),
        option('priority', 'high', 'High'),
        option('priority', 'medium', 'Medium'),
        option('priority', 'low', 'Low'),
        option('priority', 'none', 'None'),
      ],
    },
  },
  { id: 'prop-assignee', name: 'Assignee', typ: 'String', meta: null },
  { id: 'prop-description', name: 'Body', typ: 'Markdown', meta: null },
]

const clone = (value) => structuredClone(value)

const graphqlPropertyValue = (value = {}) => {
  if (typeof value !== 'object' || Array.isArray(value) || value === null) return { string: String(value ?? '') }
  if ('select' in value) return { optionId: String(value.select) }
  if ('multiSelect' in value) return { optionIds: value.multiSelect.map(String) }
  if ('relation' in value) return { dataIds: value.relation.map(String) }
  if ('integer' in value) return { number: String(value.integer) }
  if ('image' in value) return { url: String(value.image) }
  return clone(value)
}

const propertyDataEntry = (propertyId, value) => ({ propertyId, value })

const inputPropertyDataEntry = (entry) => {
  const propertyId = entry.propertyId ?? entry.property_id
  const value = graphqlPropertyValue(entry.value)
  return propertyDataEntry(
    propertyId,
    propertyId === 'prop-identifier' && value.string !== undefined
      ? { id: value.string }
      : value,
  )
}

const seedData = () => [
  {
    id: 'seed-data-201',
    name: 'Prepare release notes',
    createdAt: '2026-07-15T09:00:00.000Z',
    updatedAt: '2026-07-16T09:00:00.000Z',
    propertyData: [
      propertyDataEntry('prop-identifier', { id: 'DATA-201' }),
      propertyDataEntry('prop-status', { optionId: 'status-todo' }),
      propertyDataEntry('prop-priority', { optionId: 'priority-high' }),
      propertyDataEntry('prop-description', { markdown: 'Seed data for deterministic E2E coverage.' }),
    ],
  },
  {
    id: 'seed-data-202',
    name: 'Review content schema',
    createdAt: '2026-07-14T09:00:00.000Z',
    updatedAt: '2026-07-16T10:00:00.000Z',
    propertyData: [
      propertyDataEntry('prop-identifier', { id: 'DATA-202' }),
      propertyDataEntry('prop-status', { optionId: 'status-in_progress' }),
      propertyDataEntry('prop-priority', { optionId: 'priority-medium' }),
      propertyDataEntry('prop-description', { markdown: 'A second item keeps board and workflow tests meaningful.' }),
    ],
  },
]

let state
const rooms = new Map()
let engineOperations = []
let engineOperationSequences = new Map()
let nextEngineSequence = 1
// The real Photon Worker runs separately in the Live browser suite. Only its
// Library authorization and canonical checkpoint boundary are fixtures here.
let liveEpoch = ''
let liveDecisions = new Map()
let liveCheckpointDelayMs = 0
let liveAuthorizeDelayMs = 0

function roomFor(roomId) {
  const existing = rooms.get(roomId)
  if (existing) return existing

  const room = {
    clients: new Set(),
    doc: new Y.Doc(),
  }
  rooms.set(roomId, room)
  return room
}

function broadcastRoom(room, payload, { except } = {}) {
  for (const client of room.clients) {
    if (client === except || client.readyState !== WebSocket.OPEN) continue
    client.send(payload)
  }
}

function broadcastPresence(room) {
  broadcastRoom(room, JSON.stringify({
    type: 'presence',
    onlineCount: room.clients.size,
  }))
}

function resetRealtimeState() {
  for (const room of rooms.values()) {
    for (const client of room.clients) {
      client.terminate()
    }
    room.clients.clear()
    room.doc.destroy()
  }
  rooms.clear()
}

function resetEngineState() {
  engineOperations = []
  engineOperationSequences = new Map()
  nextEngineSequence = 1
}

function engineCursor(scope, position) {
  return {
    scope,
    remote: 'photon-engine-server',
    position,
    updated_at_ms: Date.now(),
  }
}

function pushEngineOperations(payload = {}) {
  const scope = String(payload.scope ?? '')
  const after = Number(payload.cursor?.position ?? 0)
  const decisions = []
  const requestOperationIds = new Set(
    (payload.operations ?? []).map((operation) => operation?.id).filter(Boolean),
  )

  for (const operation of payload.operations ?? []) {
    if (!operation?.id || operation.key?.scope !== scope) continue

    let remoteSequence = engineOperationSequences.get(operation.id)
    if (remoteSequence === undefined) {
      remoteSequence = nextEngineSequence++
      engineOperationSequences.set(operation.id, remoteSequence)
      engineOperations.push({
        operation: clone(operation),
        remote_sequence: remoteSequence,
      })
    }
    decisions.push({
      type: 'accepted',
      operation_id: operation.id,
      remote_sequence: remoteSequence,
    })
  }

  const synchronizedOperations = engineOperations.filter((entry) => (
    entry.operation.key?.scope === scope && entry.remote_sequence > after
  ))
  const serverOperations = synchronizedOperations.filter(
    (entry) => !requestOperationIds.has(entry.operation.id),
  )
  const cursorPosition = synchronizedOperations.reduce(
    (position, entry) => Math.max(position, entry.remote_sequence),
    after,
  )

  return {
    decisions,
    server_operations: clone(
      serverOperations.map((entry) => entry.operation),
    ),
    cursor: cursorPosition > after
      ? engineCursor(scope, cursorPosition)
      : payload.cursor ? clone(payload.cursor) : null,
  }
}

function pullEngineOperations(payload = {}) {
  const scope = String(payload.scope ?? '')
  const after = Number(payload.cursor?.position ?? 0)
  const operations = engineOperations.filter((entry) => (
    entry.operation.key?.scope === scope && entry.remote_sequence > after
  ))
  const cursorPosition = operations.reduce(
    (position, entry) => Math.max(position, entry.remote_sequence),
    after,
  )

  return {
    operations: clone(operations),
    cursor: engineCursor(scope, cursorPosition),
  }
}

function resetState() {
  liveEpoch = randomUUID()
  liveDecisions = new Map()
  state = {
    repository: clone(repositoryTemplate),
    properties: clone(propertyTemplates),
    data: seedData(),
    nextDataNumber: 103,
    nextPropertyNumber: 1,
    liveCheckpoints: [],
    dataWrites: [],
  }
  resetEngineState()
}

resetState()

function repositoryExists(org, repo) {
  return org === 'quantum-box' && repo === state.repository.username
}

function publicRepository() {
  return {
    id: state.repository.id,
    username: state.repository.username,
    name: state.repository.name,
    description: state.repository.description,
  }
}

function restRepository() {
  return {
    ...publicRepository(),
    organization_id: 'org-1',
    org_username: 'quantum-box',
  }
}

function findData(value) {
  return state.data.find((item) => {
    const identifier = item.propertyData.find((entry) => entry.propertyId === 'prop-identifier')?.value?.id
    return item.id === value || identifier === value
  })
}

function createData(input = {}) {
  const number = state.nextDataNumber++
  const now = new Date().toISOString()
  const data = {
    id: `data-${number}`,
    name: String(input.dataName ?? input.name ?? `Untitled ${number}`),
    createdAt: now,
    updatedAt: now,
    propertyData: [
      propertyDataEntry('prop-identifier', { id: `DATA-${number}` }),
      ...(input.propertyData ?? input.property_data ?? []).map(inputPropertyDataEntry),
    ],
  }
  state.data.push(data)
  return data
}

// The API treats an empty value as a clear command, which is the only way a
// patch can remove a value.
const isClearedPropertyValue = (value) =>
  Object.values(value ?? {}).every(
    (field) => field === '' || (Array.isArray(field) && field.length === 0),
  )

function updateData(input = {}) {
  const data = findData(input.dataId ?? input.id)
  if (!data) return null
  state.dataWrites.push(clone(input))
  if (input.dataName ?? input.name) data.name = String(input.dataName ?? input.name)
  const inputPropertyData = input.propertyData ?? input.property_data
  if (inputPropertyData !== undefined) {
    // updateData is a patch: a Property the payload omits keeps its stored
    // value, so the fixture must not treat the payload as a replacement set.
    for (const entry of inputPropertyData.map(inputPropertyDataEntry)) {
      const index = data.propertyData.findIndex(
        (candidate) => candidate.propertyId === entry.propertyId,
      )
      if (isClearedPropertyValue(entry.value)) {
        if (index >= 0) data.propertyData.splice(index, 1)
        continue
      }
      if (index >= 0) data.propertyData[index] = entry
      else data.propertyData.push(entry)
    }
  }
  data.updatedAt = new Date().toISOString()
  data.record_version = String(BigInt(data.record_version ?? '1') + 1n)
  return data
}

/**
 * Create the record at `dataId`, or patch the one already there.
 *
 * The route the v2 client's record writes go through: it names the record
 * itself, so it cannot tell a first write from an edit and must not be
 * answered with the 404 that `PUT .../data/{id}` gives. See
 * `apps/api/src/handler/data.rs`.
 */
function upsertData(dataId, input = {}) {
  const existing = findData(dataId)
  if (existing) return { data: updateData({ ...input, dataId }), created: false }

  const number = state.nextDataNumber++
  const now = new Date().toISOString()
  const data = {
    id: dataId,
    name: String(input.dataName ?? input.name ?? `Untitled ${number}`),
    createdAt: now,
    updatedAt: now,
    propertyData: [
      propertyDataEntry('prop-identifier', { id: `DATA-${number}` }),
      ...(input.propertyData ?? input.property_data ?? []).map(inputPropertyDataEntry),
    ],
  }
  state.data.push(data)
  return { data, created: true }
}

function deleteData(value) {
  const data = findData(value)
  if (!data) return null
  state.data = state.data.filter((candidate) => candidate !== data)
  return data.id
}

// The production Library API serializes PropertyType as SCREAMING_SNAKE_CASE
// on every surface (GraphQL `typ` and REST `property_type`), so the fixture
// must serve the same wire format to keep E2E coverage honest.
function wirePropertyType(typ) {
  return typ.replace(/([a-z])([A-Z])/g, '$1_$2').toUpperCase()
}

function wireProperties() {
  return state.properties.map((property) => ({
    ...clone(property),
    typ: wirePropertyType(property.typ),
  }))
}

function repositorySettingsProperties() {
  return wireProperties()
}

function propertyMetaFromInput(input) {
  if (input.meta?.select) {
    return {
      options: input.meta.select.map((entry, index) => ({
        id: `option-${state.nextPropertyNumber}-${index + 1}`,
        key: entry.identifier,
        name: entry.label,
      })),
    }
  }
  if (input.meta?.multiSelect) {
    return {
      options: input.meta.multiSelect.map((entry, index) => ({
        id: `option-${state.nextPropertyNumber}-${index + 1}`,
        key: entry.identifier,
        name: entry.label,
      })),
    }
  }
  if (input.meta?.relation) return { databaseId: input.meta.relation }
  if (typeof input.meta?.id === 'boolean') return { autoGenerate: input.meta.id }
  return null
}

function addProperty(input = {}) {
  const number = state.nextPropertyNumber++
  const property = {
    id: `e2e-property-${number}`,
    name: input.propertyName,
    typ: input.propertyType,
    meta: propertyMetaFromInput(input),
  }
  state.properties.push(property)
  return clone(property)
}

function updateProperty(id, input = {}) {
  const property = state.properties.find((candidate) => candidate.id === id)
  if (!property) return null
  property.name = input.propertyName
  property.typ = input.propertyType
  property.meta = propertyMetaFromInput(input)
  return clone(property)
}

function graphqlResponse(query, variables) {
  if (query.includes('LibraryClientMeOrganizations')) {
    return {
      me: {
        id: 'library-e2e-user',
        email: 'library-e2e@local.test',
        tenantIdList: ['org-1'],
        organizations: [{
          id: 'org-1',
          operatorName: 'quantum-box',
          platformTenantId: platformId,
        }],
      },
    }
  }

  if (query.includes('LibraryClientMeTenantList')) {
    return {
      me: {
        id: 'library-e2e-user',
        email: 'library-e2e@local.test',
        tenantIdList: ['org-1'],
      },
    }
  }

  if (query.includes('LibraryClientCreateRepository')) {
    const input = variables.input ?? {}
    state.repository = {
      id: `repo-${Date.now()}`,
      username: input.repoUsername,
      name: input.repoName,
      description: input.description ?? null,
      isPublic: input.isPublic ?? false,
      policies: [{ userId: input.userId, role: 'owner' }],
    }
    return {
      createRepo: {
        ...publicRepository(),
        orgUsername: input.orgUsername,
        isPublic: state.repository.isPublic,
      },
    }
  }

  if (query.includes('LibraryClientOrganizationRepos')) {
    return {
      organization: variables.org === 'quantum-box'
        ? {
            id: 'org-1',
            name: 'Quantum Box',
            username: 'quantum-box',
            repos: [publicRepository()],
          }
        : null,
    }
  }

  if (query.includes('LibraryClientRepositorySettings')) {
    const exists = repositoryExists(variables.orgUsername, variables.repoUsername)
    return {
      repo: exists ? clone(state.repository) : null,
      properties: exists ? repositorySettingsProperties() : [],
    }
  }

  if (query.includes('LibraryClientRepoData')) {
    const exists = repositoryExists(variables.org, variables.repo)
    return {
      repo: exists
        ? {
            ...publicRepository(),
            dataList: {
              items: clone(state.data),
              paginator: {
                currentPage: 1,
                itemsPerPage: variables.pageSize ?? 100,
                totalItems: state.data.length,
                totalPages: 1,
              },
            },
            properties: wireProperties(),
          }
        : null,
    }
  }

  if (query.includes('LibraryClientProperties')) {
    return {
      properties: repositoryExists(variables.org, variables.repo) ? wireProperties() : [],
    }
  }

  if (query.includes('LibraryClientDataDetail')) {
    const exists = repositoryExists(variables.org, variables.repo)
    return {
      data: exists ? clone(findData(variables.dataId) ?? null) : null,
      properties: exists ? wireProperties() : [],
    }
  }

  if (query.includes('LibraryClientAddData')) {
    return {
      addData: repositoryExists(variables.input?.orgUsername, variables.input?.repoUsername)
        ? clone(createData(variables.input))
        : null,
    }
  }

  if (query.includes('LibraryClientUpdateData')) {
    return {
      updateData: repositoryExists(variables.input?.orgUsername, variables.input?.repoUsername)
        ? clone(updateData(variables.input))
        : null,
    }
  }

  if (query.includes('LibraryClientDeleteData')) {
    return {
      deleteData: repositoryExists(variables.org, variables.repo)
        ? deleteData(variables.dataId)
        : null,
    }
  }

  if (query.includes('LibraryClientAddRepositoryProperty')) {
    return {
      addProperty: repositoryExists(variables.input?.orgUsername, variables.input?.repoUsername)
        ? addProperty(variables.input)
        : null,
    }
  }

  if (query.includes('LibraryClientUpdateRepositoryProperty')) {
    return {
      updateProperty: repositoryExists(variables.input?.orgUsername, variables.input?.repoUsername)
        ? updateProperty(variables.id, variables.input)
        : null,
    }
  }

  if (query.includes('LibraryClientDeleteRepositoryProperty')) {
    if (!repositoryExists(variables.orgUsername, variables.repoUsername)) {
      return { deleteProperty: null }
    }
    const before = state.properties.length
    state.properties = state.properties.filter((property) => property.id !== variables.id)
    return { deleteProperty: state.properties.length === before ? null : variables.id }
  }

  if (query.includes('LibraryClientUpdateRepository')) {
    const input = variables.input ?? {}
    if (!repositoryExists(input.orgUsername, input.repoUsername)) return { updateRepo: null }
    if (typeof input.description === 'string') state.repository.description = input.description
    if (typeof input.isPublic === 'boolean') state.repository.isPublic = input.isPublic
    return { updateRepo: clone(state.repository) }
  }

  return null
}

function restData(item) {
  return {
    id: item.id,
    name: item.name,
    items: item.propertyData.map((entry) => ({
      property_id: entry.propertyId,
      key: entry.propertyId,
      value: clone(entry.value),
    })),
  }
}

function sendJson(response, status, body) {
  response.writeHead(status, {
    'access-control-allow-headers': 'authorization, content-type, x-operator-id, x-platform-id',
    'access-control-allow-methods': 'DELETE, GET, OPTIONS, POST, PUT',
    'access-control-allow-origin': '*',
    'cache-control': 'no-store',
    'content-type': 'application/json; charset=utf-8',
  })
  response.end(JSON.stringify(body))
}

async function readJson(request) {
  const chunks = []
  for await (const chunk of request) chunks.push(chunk)
  if (chunks.length === 0) return {}
  return JSON.parse(Buffer.concat(chunks).toString('utf8'))
}

const server = createServer(async (request, response) => {
  try {
    if (request.method === 'OPTIONS') {
      sendJson(response, 204, null)
      return
    }

    const url = new URL(request.url ?? '/', `http://${host}:${port}`)

    if (request.method === 'GET' && url.pathname === '/__e2e/health') {
      sendJson(response, 200, { service: 'library-e2e-api', status: 'ok' })
      return
    }

    if (request.method === 'POST' && url.pathname === '/__e2e/reset') {
      liveCheckpointDelayMs = 0
      liveAuthorizeDelayMs = 0
      resetRealtimeState()
      resetState()
      sendJson(response, 200, { service: 'library-e2e-api', status: 'reset' })
      return
    }

    if (request.method === 'POST' && url.pathname === '/__e2e/live-delay') {
      const input = await readJson(request)
      liveCheckpointDelayMs = Math.min(5000, Math.max(0, Number(input.checkpointMs) || 0))
      liveAuthorizeDelayMs = Math.min(5000, Math.max(0, Number(input.authorizeMs) || 0))
      sendJson(response, 200, { checkpointMs: liveCheckpointDelayMs })
      return
    }

    if (request.method === 'GET' && url.pathname === '/__e2e/state') {
      sendJson(response, 200, state)
      return
    }

    if (request.method === 'GET' && url.pathname === '/__e2e/engine') {
      sendJson(response, 200, { operations: clone(engineOperations) })
      return
    }

    if (request.method === 'GET' && url.pathname === '/v1beta/repos') {
      sendJson(response, 200, [restRepository()])
      return
    }

    if (request.method === 'GET' && url.pathname === '/api/health') {
      sendJson(response, 200, { status: 'ok' })
      return
    }

    if (request.method === 'POST' && url.pathname === '/api/engine/push') {
      sendJson(response, 200, pushEngineOperations(await readJson(request)))
      return
    }

    if (request.method === 'POST' && url.pathname === '/api/engine/pull') {
      sendJson(response, 200, pullEngineOperations(await readJson(request)))
      return
    }

    if (request.method === 'POST' && url.pathname === '/v1/graphql') {
      const { query = '', variables = {} } = await readJson(request)
      const data = graphqlResponse(query, variables)
      if (data === null) {
        sendJson(response, 400, { errors: [{ message: 'Unsupported E2E GraphQL operation' }] })
        return
      }
      sendJson(response, 200, { data })
      return
    }

    const liveMatch = url.pathname.match(/^\/v1beta\/repos\/([^/]+)\/([^/]+)\/data\/([^/]+)\/live\/(authorize|checkpoint)$/)
    if (request.method === 'POST' && liveMatch) {
      const [, org, repo, dataId, action] = liveMatch
      const actorId = request.headers.authorization === 'Bearer dev:local'
        ? 'library-e2e-user'
        : request.headers.authorization === 'Bearer dev:ren' ? 'library-e2e-ren' : null
      if (!actorId) {
        sendJson(response, 403, { message: 'Live editing is not authorized' })
        return
      }
      const data = findData(dataId)
      if (!repositoryExists(org, repo) || !data) {
        sendJson(response, 404, { message: 'Data not found' })
        return
      }
      const input = await readJson(request)
      if (action === 'checkpoint' && liveCheckpointDelayMs > 0) {
        await new Promise((resolve) => setTimeout(resolve, liveCheckpointDelayMs))
      }
      const property = state.properties.find((entry) => entry.id === input.property_id)
      if (!property || !['Markdown', 'RichText'].includes(property.typ)) {
        sendJson(response, 404, { message: 'Live document Property not found' })
        return
      }
      const format = property.typ === 'RichText' ? 'richText' : 'markdown'
      const body = data.propertyData.find((entry) => entry.propertyId === property.id)?.value[format] ?? ''
      const recordVersion = data.record_version ?? '1'
      if (action === 'authorize') {
        if (liveAuthorizeDelayMs > 0) await new Promise((resolve) => setTimeout(resolve, liveAuthorizeDelayMs))
        sendJson(response, 200, {
          tenant_id: platformId,
          database_id: `fixture-database-${liveEpoch}`,
          data_id: data.id,
          property_id: property.id,
          actor_id: actorId,
          room_id: `${platformId}:fixture-database-${liveEpoch}:${data.id}:${property.id}`,
          format,
          body,
          record_version: recordVersion,
        })
        return
      }
      if (input.format !== format || typeof input.body !== 'string' || typeof input.expected_record_version !== 'string') {
        state.liveCheckpoints.push({ ...input, status: 422 })
        sendJson(response, 422, { message: 'Invalid Live checkpoint payload' })
        return
      }
      const fingerprint = JSON.stringify({ dataId, ...input })
      const prior = liveDecisions.get(input.operation_id)
      if (prior) {
        sendJson(response, prior.fingerprint === fingerprint ? prior.status : 409,
          prior.fingerprint === fingerprint ? prior.result : { message: 'Operation id reused' })
        return
      }
      if (input.expected_record_version !== recordVersion) {
        state.liveCheckpoints.push({ ...input, status: 409, current_version: recordVersion })
        sendJson(response, 409, { message: 'Record changed', record_version: recordVersion })
        return
      }
      updateData({
        dataId,
        propertyData: [{ propertyId: property.id, value: { [format]: input.body } }],
      })
      const result = { record_version: data.record_version }
      state.liveCheckpoints.push({ ...input, status: 200, record_version: data.record_version })
      liveDecisions.set(input.operation_id, { fingerprint, status: 200, result })
      sendJson(response, 200, result)
      return
    }

    const repoMatch = url.pathname.match(/^\/v1beta\/repos\/([^/]+)\/([^/]+)(?:\/(.*))?$/)
    if (repoMatch) {
      const [, encodedOrg, encodedRepo, suffix = ''] = repoMatch
      const org = decodeURIComponent(encodedOrg)
      const repo = decodeURIComponent(encodedRepo)
      if (!repositoryExists(org, repo)) {
        sendJson(response, 404, { error: 'Repository not found' })
        return
      }

      if (request.method === 'GET' && suffix === '') {
        sendJson(response, 200, {
          ...restRepository(),
          is_public: state.repository.isPublic ?? false,
        })
        return
      }

      if (request.method === 'GET' && suffix === 'properties') {
        sendJson(response, 200, state.properties.map((property) => ({
          id: property.id,
          name: property.name,
          property_type: wirePropertyType(property.typ),
        })))
        return
      }

      if (request.method === 'GET' && suffix === 'data-list') {
        sendJson(response, 200, {
          data: state.data.map(restData),
          paginator: {
            current_page: 1,
            items_per_page: state.data.length,
            total_items: state.data.length,
            total_pages: 1,
          },
        })
        return
      }

      if (request.method === 'POST' && suffix === 'data') {
        sendJson(response, 201, restData(createData(await readJson(request))))
        return
      }

      // Matched before the update-only route below, which would otherwise
      // read "{id}/upsert" as the record id and answer 404.
      const upsertMatch = suffix.match(/^data\/(.+)\/upsert$/)
      if (upsertMatch && request.method === 'PUT') {
        const dataId = decodeURIComponent(upsertMatch[1])
        const { data, created } = upsertData(dataId, await readJson(request))
        sendJson(response, created ? 201 : 200, restData(data))
        return
      }

      const dataMatch = suffix.match(/^data\/(.+)$/)
      if (dataMatch) {
        const dataId = decodeURIComponent(dataMatch[1])
        if (request.method === 'GET') {
          const data = findData(dataId)
          sendJson(response, data ? 200 : 404, data ? restData(data) : { error: 'Data not found' })
          return
        }
        if (request.method === 'PUT') {
          const data = updateData({ ...(await readJson(request)), dataId })
          sendJson(response, data ? 200 : 404, data ? restData(data) : { error: 'Data not found' })
          return
        }
        if (request.method === 'DELETE') {
          const deleted = deleteData(dataId)
          sendJson(response, deleted ? 200 : 404, { id: deleted })
          return
        }
      }
    }

    sendJson(response, 404, { error: 'Not found' })
  } catch (error) {
    sendJson(response, 500, {
      error: error instanceof Error ? error.message : 'E2E fixture failure',
    })
  }
})

const websocketServer = new WebSocketServer({ noServer: true })

websocketServer.on('connection', (client, request) => {
  const url = new URL(request.url ?? '/ws', `http://${host}:${port}`)
  const roomId = url.searchParams.get('room')?.trim() || 'default'
  const room = roomFor(roomId)
  room.clients.add(client)

  client.send(Y.encodeStateAsUpdate(room.doc))
  broadcastPresence(room)

  client.on('message', (payload, isBinary) => {
    if (isBinary) {
      const update = new Uint8Array(payload.buffer, payload.byteOffset, payload.byteLength)
      try {
        Y.applyUpdate(room.doc, update)
      } catch {
        return
      }
      broadcastRoom(room, payload, { except: client })
      return
    }

    try {
      const message = JSON.parse(payload.toString())
      if (message?.type === 'awareness') {
        broadcastRoom(room, payload.toString(), { except: client })
      }
    } catch {
      // Ignore text outside the fixture's awareness protocol.
    }
  })

  client.on('close', () => {
    if (!room.clients.delete(client)) return
    broadcastPresence(room)
  })
})

server.on('upgrade', (request, socket, head) => {
  const url = new URL(request.url ?? '/', `http://${host}:${port}`)
  if (url.pathname !== '/ws') {
    socket.write('HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n')
    socket.destroy()
    return
  }

  websocketServer.handleUpgrade(request, socket, head, (client) => {
    websocketServer.emit('connection', client, request)
  })
})

server.listen(port, host, () => {
  console.log(`Library E2E API fixture listening on http://${host}:${port}`)
})

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    resetRealtimeState()
    websocketServer.close()
    server.close(() => process.exit(0))
  })
}
