import { act, render } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import { RecordBodyEditor } from './RecordBodyEditor'
import type { PhotonLiveState } from '../lib/photonLive'

/**
 * The editor, its Yjs fragments and the rooms are stand-ins: what each
 * fragment "holds" is a plain string in `bodies`. The decisions themselves
 * are covered by liveBody.test.ts; this file checks the wiring -- where a
 * commit goes, how the editor is rebound, and what the page shows.
 */
const mocks = vi.hoisted(() => {
  const bodies = new Map<unknown, string>()
  const editor = {
    document: [] as { body: string }[],
    tryParseMarkdownToBlocks: vi.fn((value: string) => [{ body: value }]),
    tryParseHTMLToBlocks: vi.fn((value: string) => [{ body: value }]),
    replaceBlocks: vi.fn(),
    blocksToMarkdownLossy: vi.fn((blocks: { body?: string }[]) => blocks[0]?.body ?? ''),
    blocksToHTMLLossy: vi.fn(() => ''),
    prosemirrorView: undefined as undefined | { composing: boolean },
    unregisterExtension: vi.fn(),
    registerExtension: vi.fn(),
  }
  return {
    bodies,
    editor,
    boundFragment: null as unknown,
    onEditorChange: undefined as undefined | ((editorValue: typeof editor) => void),
    rooms: [] as Array<{
      fragment: unknown
      queued: string[]
      set(next: Partial<PhotonLiveState>): void
      ready(): void
    }>,
    createdWithCollaboration: [] as boolean[],
  }
})

vi.mock('@blocknote/react', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@blocknote/react')>()),
  useCreateBlockNote: (options: { collaboration?: { fragment: unknown } }) => {
    mocks.createdWithCollaboration.push(options.collaboration !== undefined)
    if (options.collaboration && mocks.boundFragment === null) {
      mocks.boundFragment = options.collaboration.fragment
    }
    return mocks.editor
  },
  useEditorChange: (onChange: typeof mocks.onEditorChange) => {
    mocks.onEditorChange = onChange
  },
}))

vi.mock('@blocknote/shadcn', () => ({
  BlockNoteView: () => <div data-testid="block-note-view" />,
}))

vi.mock('@blocknote/core/yjs', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@blocknote/core/yjs')>()),
  yXmlFragmentToBlocks: (_editor: unknown, fragment: unknown) => [
    { body: mocks.bodies.get(fragment) ?? 'Original' },
  ],
  blocksToYXmlFragment: (_editor: unknown, blocks: { body: string }[], fragment: unknown) => {
    mocks.bodies.set(fragment, blocks[0]?.body ?? '')
  },
}))

vi.mock('@blocknote/core', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@blocknote/core')>()
  return {
    ...actual,
    YSyncExtension: (options: { fragment: unknown }) => {
      mocks.boundFragment = options.fragment
      return { key: 'ySync' }
    },
    YCursorExtension: () => ({ key: 'yCursor' }),
    YUndoExtension: () => ({ key: 'yUndo' }),
  }
})

vi.mock('../app/kitConfig', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../app/kitConfig')>()
  return {
    ...actual,
    appKitConfig: {
      ...actual.appKitConfig,
      dataLive: { ...actual.appKitConfig.dataLive, baseUrl: 'https://live.test' },
    },
  }
})

vi.mock('../lib/photonLive/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/photonLive/client')>()
  return {
    ...actual,
    createPhotonLiveProvider: () => {
      const doc = new Y.Doc()
      const listeners = new Set<(state: PhotonLiveState) => void>()
      let state: PhotonLiveState = {
        status: 'authorizing',
        saveStatus: 'idle',
        error: null,
        initialized: false,
        version: 0,
        recordVersion: '1',
        hasUnackedChanges: false,
        canEdit: false,
      }
      const room = {
        doc,
        fragment: doc.getXmlFragment('prosemirror'),
        awareness: {},
        user: { name: 'Aoi', color: '#000' },
        session: null,
        roomGeneration: null,
        queued: [] as string[],
        getState: () => state,
        unsentBody: () => null,
        subscribe(listener: (next: PhotonLiveState) => void) {
          listeners.add(listener)
          listener(state)
          return () => listeners.delete(listener)
        },
        subscribeSave(listener: (next: PhotonLiveState) => void) {
          return room.subscribe(listener)
        },
        queueCheckpoint(body: string) {
          room.queued.push(body)
        },
        flushCheckpoint: vi.fn(),
        detach: vi.fn(),
        destroy: vi.fn(),
        set(next: Partial<PhotonLiveState>) {
          state = { ...state, ...next }
          listeners.forEach((listener) => listener(state))
        },
        ready() {
          room.set({ status: 'connected', initialized: true, canEdit: true })
        },
      }
      mocks.rooms.push(room)
      return room
    },
  }
})

const liveTarget = {
  org: 'quantum-box',
  repo: 'photon-core',
  dataId: 'seed-data-201',
  propertyId: 'prop-description',
}

/** The person types: the bound fragment and the editor document change. */
function type(body: string) {
  mocks.bodies.set(mocks.boundFragment, body)
  mocks.editor.document = [{ body }]
  act(() => mocks.onEditorChange?.(mocks.editor))
}

async function debounce() {
  await act(async () => new Promise((resolve) => setTimeout(resolve, 600)))
}

function renderLive(onCommit = vi.fn(() => Promise.resolve(true))) {
  const view = render(
    <RecordBodyEditor
      value="Original"
      format="markdown"
      onCommit={onCommit}
      liveTarget={liveTarget}
    />,
  )
  return { ...view, onCommit }
}

describe('RecordBodyEditor with Photon Live', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.bodies.clear()
    mocks.boundFragment = null
    mocks.onEditorChange = undefined
    mocks.editor.document = [{ body: 'Original' }]
    mocks.editor.prosemirrorView = undefined
    mocks.rooms = []
    mocks.createdWithCollaboration = []
  })

  it('mounts an editable body on a local draft before any room has answered', async () => {
    const { getByTestId, container } = renderLive()
    await act(async () => Promise.resolve())

    expect(getByTestId('block-note-view')).toBeTruthy()
    // One editor, created once, on the local draft -- never swapped later.
    expect(mocks.createdWithCollaboration).not.toContain(false)
    expect(mocks.rooms).toHaveLength(1)
    expect(container.querySelector('.record-body-blocknote')?.getAttribute('data-live-collab')).toBeNull()
  })

  it('joins a room that answers before anything is typed and checkpoints through it', async () => {
    const { container, onCommit } = renderLive()
    await act(async () => Promise.resolve())

    act(() => mocks.rooms[0].ready())
    expect(mocks.editor.registerExtension).toHaveBeenCalled()
    expect(mocks.boundFragment).toBe(mocks.rooms[0].fragment)
    expect(container.querySelector('.record-body-blocknote')?.getAttribute('data-live-collab')).toBe('on')

    type('Shared edit')
    await debounce()
    expect(mocks.rooms[0].queued).toContain('Shared edit')
    expect(onCommit).not.toHaveBeenCalled()
  })

  it('carries an edit typed before the room answered into the room instead of saving over it', async () => {
    const { container, onCommit, getByTestId } = renderLive()
    await act(async () => Promise.resolve())

    type('Typed while connecting')
    await debounce()
    // Held for the room, not saved over the canonical body under it.
    expect(onCommit).not.toHaveBeenCalled()
    expect(getByTestId('data-editor-live-status').textContent).toBe('Saving')

    act(() => mocks.rooms[0].ready())
    expect(mocks.bodies.get(mocks.rooms[0].fragment)).toBe('Typed while connecting')
    expect(mocks.boundFragment).toBe(mocks.rooms[0].fragment)
    expect(mocks.rooms[0].queued).toContain('Typed while connecting')
    expect(container.querySelector('.record-body-blocknote')?.getAttribute('data-live-collab')).toBe('on')
    expect(onCommit).not.toHaveBeenCalled()
  })

  it('keeps saving normally when Live is unavailable', async () => {
    const { onCommit, getByTestId } = renderLive()
    await act(async () => Promise.resolve())

    const { PhotonLiveError } = await import('../lib/photonLive')
    act(() => mocks.rooms[0].set({
      status: 'failed',
      error: new PhotonLiveError('Live is disabled', 'disabled', 404),
    }))
    type('Typed after Live refused')
    await debounce()
    expect(onCommit).toHaveBeenCalledWith('Typed after Live refused')
    expect(getByTestId('data-editor-live-status').textContent).toBe(
      'Shared editing is unavailable. Edits are saved normally',
    )
  })

  it('reports a conflict, keeps the draft, and saves the next edit normally', async () => {
    const { container, onCommit, getByTestId } = renderLive()
    await act(async () => Promise.resolve())
    act(() => mocks.rooms[0].ready())

    type('Draft in the room')
    await debounce()
    act(() => mocks.rooms[0].set({ status: 'failed', saveStatus: 'conflict' }))
    await act(async () => new Promise((resolve) => setTimeout(resolve, 1_100)))
    expect(mocks.rooms).toHaveLength(2)
    mocks.bodies.set(mocks.rooms[1].fragment, 'External body')
    act(() => mocks.rooms[1].ready())

    expect(getByTestId('data-editor-live-status').textContent).toBe(
      'Shared editing stopped after a conflicting change. Edits are saved normally',
    )
    expect(container.querySelector('.record-body-blocknote')?.getAttribute('data-live-collab')).toBe('off')
    expect(onCommit).not.toHaveBeenCalled()

    type('Draft in the room, kept')
    await debounce()
    expect(onCommit).toHaveBeenCalledWith('Draft in the room, kept')
  })

  it('says nothing while the room is healthy and idle', async () => {
    const { queryByTestId } = renderLive()
    await act(async () => Promise.resolve())
    expect(queryByTestId('data-editor-live-status')).toBeNull()
    act(() => mocks.rooms[0].ready())
    expect(queryByTestId('data-editor-live-status')).toBeNull()
  })

  it('shows a reconnecting room as offline, not as unavailable', async () => {
    const { getByTestId } = renderLive()
    await act(async () => Promise.resolve())
    act(() => mocks.rooms[0].ready())
    const { PhotonLiveError } = await import('../lib/photonLive')
    act(() => mocks.rooms[0].set({
      status: 'failed',
      error: new PhotonLiveError('timeout', 'timeout', undefined, true),
    }))
    expect(getByTestId('data-editor-live-status').textContent).toBe(
      'Offline — your edits sync when the connection returns',
    )
  })
})
