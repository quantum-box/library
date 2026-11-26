import DOMPurify from 'dompurify'

export type DiagramKind = 'mermaid' | 'plantuml'

/**
 * Rendering diagrams, shared by every preview on the page.
 *
 * A preview lives inside a BlockNote node view, and node views are rebuilt
 * whenever ProseMirror re-renders their block: a Live room rebinding, a
 * remote update, a re-seed of a read-only body. Each rebuild used to start
 * the diagram over and drop a render already in flight, so a block rebuilt
 * more often than a slow phone finishes Mermaid never showed anything while
 * it kept the main thread busy. Results therefore outlive the preview that
 * asked for them: a render in flight runs to completion, and the rebuilt
 * preview picks the same result up by its source.
 */

const CACHE_LIMIT = 64
/** A render that has not settled by then is reported as failed. */
export const DIAGRAM_RENDER_TIMEOUT_MS = 20_000

const cache = new Map<string, string>()
const inflight = new Map<string, Job>()

interface Job {
  promise: Promise<string>
  /** Previews still waiting for this result. */
  waiters: number
  started: boolean
}

// Mermaid has global configuration and a shared rendering queue. Keep
// initialization and rendering together so concurrent blocks cannot change
// each other's theme. Running one at a time also keeps a page full of
// diagrams from monopolising the main thread.
let mermaidQueue: Promise<unknown> = Promise.resolve()
let renderCount = 0

const keyOf = (kind: DiagramKind, source: string) => `${kind}\n${source}`

/** A finished result for this source, if one has been rendered. */
export function cachedDiagram(kind: DiagramKind, source: string): string | undefined {
  const key = keyOf(kind, source)
  const svg = cache.get(key)
  if (svg !== undefined) {
    // Keep recently shown diagrams at the young end of the LRU.
    cache.delete(key)
    cache.set(key, svg)
  }
  return svg
}

/**
 * Render a diagram to sanitized SVG.
 *
 * `signal` only withdraws this caller. The render itself is abandoned only
 * when nobody is waiting for it before it starts; once started it finishes
 * and is cached, because the block that asked is usually about to come back.
 */
export function renderDiagram(kind: DiagramKind, source: string, signal: AbortSignal): Promise<string> {
  const cached = cachedDiagram(kind, source)
  if (cached !== undefined) return Promise.resolve(cached)
  const key = keyOf(kind, source)
  let job = inflight.get(key)
  if (!job) {
    const created: Job = { promise: Promise.resolve(''), waiters: 0, started: false }
    const run = async () => {
      // Let input, scrolling and painting in between two diagrams. This also
      // lets the caller register before the waiter count is read.
      await yieldToMain()
      if (created.waiters === 0) throw abortError()
      created.started = true
      const raw = await withTimeout(kind === 'mermaid' ? renderMermaid(source) : renderPlantUml(source))
      return DOMPurify.sanitize(raw, { USE_PROFILES: { svg: true, svgFilters: true } })
    }
    const queued = kind === 'mermaid'
      ? (mermaidQueue = mermaidQueue.then(run, run))
      : run()
    created.promise = (queued as Promise<string>).then(
      (svg) => {
        inflight.delete(key)
        remember(key, svg)
        return svg
      },
      (error: unknown) => {
        inflight.delete(key)
        throw error
      },
    )
    // Nobody may be listening by the time it settles.
    created.promise.catch(() => {})
    inflight.set(key, created)
    job = created
  }
  return waitFor(job, signal)
}

function waitFor(job: Job, signal: AbortSignal): Promise<string> {
  if (signal.aborted) return Promise.reject(abortError())
  job.waiters += 1
  return new Promise<string>((resolve, reject) => {
    const leave = () => {
      job.waiters -= 1
      signal.removeEventListener('abort', onAbort)
    }
    const onAbort = () => {
      leave()
      reject(abortError())
    }
    signal.addEventListener('abort', onAbort)
    job.promise.then(
      (svg) => { if (!signal.aborted) { leave(); resolve(svg) } },
      (error: unknown) => { if (!signal.aborted) { leave(); reject(error) } },
    )
  })
}

async function renderMermaid(source: string): Promise<string> {
  const mermaid = (await import('mermaid')).default
  mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', suppressErrorRendering: true, htmlLabels: false })
  // A fresh id per render. Mermaid removes any element that already carries
  // the id it is given, which would delete a diagram already on the page.
  renderCount += 1
  return (await mermaid.render(`library-diagram-${renderCount}`, source)).svg
}

async function renderPlantUml(source: string): Promise<string> {
  const base = (import.meta.env.VITE_KROKI_BASE_URL || 'https://kroki.io').replace(/\/+$/, '')
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), DIAGRAM_RENDER_TIMEOUT_MS)
  try {
    const response = await fetch(`${base}/plantuml/svg`, {
      method: 'POST', headers: { 'Content-Type': 'text/plain' },
      body: source, signal: controller.signal,
    })
    if (!response.ok) throw new Error(`Kroki: ${response.status}`)
    return await response.text()
  } finally {
    clearTimeout(timer)
  }
}

function remember(key: string, svg: string) {
  cache.set(key, svg)
  while (cache.size > CACHE_LIMIT) {
    const oldest = cache.keys().next().value
    if (oldest === undefined) break
    cache.delete(oldest)
  }
}

function withTimeout<T>(work: Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Diagram render timed out')), DIAGRAM_RENDER_TIMEOUT_MS)
    work.then(
      (value) => { clearTimeout(timer); resolve(value) },
      (error: unknown) => { clearTimeout(timer); reject(error) },
    )
  })
}

function yieldToMain() {
  return new Promise<void>((resolve) => setTimeout(resolve, 0))
}

function abortError() {
  return new DOMException('Diagram preview left', 'AbortError')
}

export function isAbortError(error: unknown) {
  return error instanceof DOMException && error.name === 'AbortError'
}

/** Test seam: forget every result and render. */
export function resetDiagramRendererForTests() {
  cache.clear()
  inflight.clear()
  mermaidQueue = Promise.resolve()
}
