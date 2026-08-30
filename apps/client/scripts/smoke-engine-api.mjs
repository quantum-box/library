const apiBaseUrl = process.env.PHOTON_ENGINE_SMOKE_URL ?? 'http://127.0.0.1:3001'
// `tenant:{tenant}:workspace:{workspace}` is the only scope shape Photon's HTTP
// boundary accepts -- anything else is a 400 before the operation is read. The
// tenant has to match the server's: `library` for library-api (see
// LIBRARY_PHOTON_ENGINE_TENANT), `photon` for a bare Photon server.
const scope =
  process.env.PHOTON_ENGINE_SMOKE_SCOPE ?? 'tenant:library:workspace:smoke'
// library-api requires an authenticated caller on /api/engine/*; a bare Photon
// server does not. Set this to a Cognito access token when pointing at a
// library-api deployment.
const authToken = process.env.PHOTON_ENGINE_SMOKE_TOKEN
const collection = process.env.PHOTON_ENGINE_SMOKE_COLLECTION ?? 'smoke_records'
const recordId = `smoke-${Date.now()}`
const actorId = process.env.PHOTON_ENGINE_SMOKE_ACTOR ?? 'smoke-client'
const operationId = `op_${crypto.randomUUID()}`
const wallTimeMs = Date.now()

function authHeaders(base = {}) {
  return authToken ? { ...base, authorization: `Bearer ${authToken}` } : base
}

async function requestJson(path, body) {
  const response = await fetch(`${apiBaseUrl}${path}`, {
    method: 'POST',
    headers: authHeaders({ 'content-type': 'application/json' }),
    body: JSON.stringify(body),
  })
  const text = await response.text()
  if (!response.ok) {
    throw new Error(`${path} returned ${response.status}: ${text}`)
  }
  return text ? JSON.parse(text) : null
}

const operation = {
  id: operationId,
  key: {
    scope,
    collection,
    record_id: recordId,
  },
  actor_id: actorId,
  timestamp: {
    wall_time_ms: wallTimeMs,
    counter: 0,
    actor_id: actorId,
  },
  kind: {
    type: 'upsert',
    value: {
      id: recordId,
      title: 'Photon Engine smoke record',
      source: 'scripts/smoke-engine-api.mjs',
    },
  },
  metadata: {
    smoke: true,
  },
}

const health = await fetch(`${apiBaseUrl}/api/health`, {
  headers: authHeaders(),
})
if (!health.ok) {
  throw new Error(`/api/health returned ${health.status}`)
}

const push = await requestJson('/api/engine/push', {
  scope,
  operations: [operation],
  cursor: null,
})

const accepted = push.decisions?.find(
  (decision) => decision.type === 'accepted' && decision.operation_id === operationId
)
if (!accepted) {
  throw new Error(`push did not accept ${operationId}: ${JSON.stringify(push)}`)
}

const pull = await requestJson('/api/engine/pull', {
  scope,
  cursor: null,
})

const pulled = pull.operations?.find(
  (entry) => entry.operation?.id === operationId && entry.remote_sequence === accepted.remote_sequence
)
if (!pulled) {
  throw new Error(`pull did not return ${operationId}: ${JSON.stringify(pull)}`)
}

console.log(JSON.stringify({
  ok: true,
  apiBaseUrl,
  authenticated: Boolean(authToken),
  scope,
  collection,
  recordId,
  operationId,
  remoteSequence: accepted.remote_sequence,
}))
