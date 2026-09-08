import { act, render, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { RecordBodyEditor } from './RecordBodyEditor'
import type { PhotonLiveProvider, PhotonLiveState } from '../lib/photonLive'

const mocks = vi.hoisted(() => {
  const editor = {
    document: [{ id: 'body' }] as unknown[],
    tryParseMarkdownToBlocks: vi.fn((value: string) => [{ id: value || 'empty' }]),
    tryParseHTMLToBlocks: vi.fn((value: string) => [{ id: `html:${value}` }]),
    replaceBlocks: vi.fn(),
    blocksToMarkdownLossy: vi.fn(() => ''),
    blocksToHTMLLossy: vi.fn(() => ''),
    prosemirrorView: undefined as undefined | { composing: boolean },
  }
  return {
    editor,
    onEditorChange: undefined as undefined | ((editorValue: typeof editor) => void),
    live: {
      provider: null as PhotonLiveProvider | null,
      state: null as PhotonLiveState | null,
      mounted: false,
      initialError: null,
    },
    queueCheckpoint: vi.fn(),
    collaborationSeen: [] as boolean[],
  }
})

vi.mock('@blocknote/react', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@blocknote/react')>()),
  useCreateBlockNote: (options: { collaboration?: unknown }) => {
    mocks.collaborationSeen.push(options.collaboration !== undefined)
    return mocks.editor
  },
  useEditorChange: (onChange: typeof mocks.onEditorChange) => {
    mocks.onEditorChange = onChange
  },
}))

vi.mock('@blocknote/shadcn', () => ({
  BlockNoteView: () => <div data-testid="block-note-view" />,
}))

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

vi.mock('../lib/photonLive/usePhotonLiveRecord', () => ({
  usePhotonLiveRecord: (options: unknown) =>
    options === null
      ? { provider: null, state: null, mounted: false, initialError: null }
      : mocks.live,
}))

const liveTarget = {
  org: 'quantum-box',
  repo: 'photon-core',
  dataId: 'seed-data-201',
  propertyId: 'prop-description',
}

function connectedState(overrides: Partial<PhotonLiveState> = {}): PhotonLiveState {
  return {
    status: 'connected',
    saveStatus: 'idle',
    error: null,
    initialized: true,
    version: 1,
    recordVersion: '1',
    hasUnackedChanges: false,
    canEdit: true,
    ...overrides,
  }
}

function connect(state: PhotonLiveState = connectedState()) {
  mocks.live = {
    provider: {
      queueCheckpoint: mocks.queueCheckpoint,
      flushCheckpoint: vi.fn(),
      fragment: {},
      awareness: {},
      user: { name: 'Aoi', color: '#000' },
    } as unknown as PhotonLiveProvider,
    state,
    mounted: true,
    initialError: null,
  }
}

describe('RecordBodyEditor with Photon Live', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.onEditorChange = undefined
    mocks.editor.prosemirrorView = undefined
    mocks.collaborationSeen = []
    mocks.live = { provider: null, state: null, mounted: false, initialError: null }
  })

  it('mounts an editable body before the room has answered', async () => {
    const onCommit = vi.fn()
    const { getByTestId } = render(
      <RecordBodyEditor
        value="Original"
        format="markdown"
        onCommit={onCommit}
        liveTarget={liveTarget}
      />,
    )

    expect(getByTestId('block-note-view')).toBeTruthy()
    expect(mocks.collaborationSeen).toEqual([false])
    await waitFor(() => expect(mocks.editor.replaceBlocks).toHaveBeenCalled())
    await act(async () => Promise.resolve())

    // Editable, and persisting through the ordinary body while it waits.
    mocks.editor.blocksToMarkdownLossy.mockReturnValue('Typed while connecting')
    act(() => mocks.onEditorChange?.(mocks.editor))
    await act(async () => new Promise((resolve) => setTimeout(resolve, 600)))
    expect(onCommit).toHaveBeenCalledWith('Typed while connecting')
    expect(mocks.queueCheckpoint).not.toHaveBeenCalled()
  })

  it('keeps the ordinary editor when the room answers after the first keystroke', async () => {
    const onLivePolicyChange = vi.fn()
    const { rerender } = render(
      <RecordBodyEditor
        value="Original"
        format="markdown"
        onCommit={vi.fn()}
        liveTarget={liveTarget}
        onLivePolicyChange={onLivePolicyChange}
      />,
    )
    await act(async () => Promise.resolve())

    mocks.editor.blocksToMarkdownLossy.mockReturnValue('Typed first')
    act(() => mocks.onEditorChange?.(mocks.editor))

    connect()
    rerender(
      <RecordBodyEditor
        value="Original"
        format="markdown"
        onCommit={vi.fn()}
        liveTarget={liveTarget}
        onLivePolicyChange={onLivePolicyChange}
      />,
    )
    await act(async () => Promise.resolve())

    // Never swapped onto a collaborative editor, so nothing was taken away
    // from under the caret, and the body still reaches the REST save.
    expect(mocks.collaborationSeen).not.toContain(true)
    expect(onLivePolicyChange).toHaveBeenLastCalledWith('fallback-editable', null)
  })

  it('joins the room when it answers before anything is typed', async () => {
    const onLivePolicyChange = vi.fn()
    const props = {
      value: 'Original',
      format: 'markdown' as const,
      onCommit: vi.fn(),
      liveTarget,
      onLivePolicyChange,
    }
    const { rerender } = render(<RecordBodyEditor {...props} />)
    await act(async () => Promise.resolve())

    connect()
    rerender(<RecordBodyEditor {...props} />)
    await act(async () => Promise.resolve())

    expect(mocks.collaborationSeen.at(-1)).toBe(true)
    expect(onLivePolicyChange).toHaveBeenLastCalledWith('live', mocks.live.state)

    mocks.editor.blocksToMarkdownLossy.mockReturnValue('Shared edit')
    act(() => mocks.onEditorChange?.(mocks.editor))
    await act(async () => new Promise((resolve) => setTimeout(resolve, 600)))
    expect(mocks.queueCheckpoint).toHaveBeenCalledWith('Shared edit')
  })

  it('keeps saving through the ordinary body after the room conflicts', async () => {
    const onCommit = vi.fn()
    const onLivePolicyChange = vi.fn()
    const props = {
      value: 'Original',
      format: 'markdown' as const,
      onCommit,
      liveTarget,
      onLivePolicyChange,
    }
    const { rerender } = render(<RecordBodyEditor {...props} />)
    await act(async () => Promise.resolve())

    connect()
    rerender(<RecordBodyEditor {...props} />)
    await act(async () => Promise.resolve())

    connect(connectedState({ status: 'failed', saveStatus: 'conflict', canEdit: false }))
    rerender(<RecordBodyEditor {...props} />)
    await act(async () => Promise.resolve())

    // The editor is not remounted -- that would drop the Y.Doc the person is
    // looking at -- but its saves go back to the durable body.
    expect(mocks.collaborationSeen.at(-1)).toBe(true)
    expect(onLivePolicyChange).toHaveBeenLastCalledWith('fallback-editable', mocks.live.state)

    mocks.editor.blocksToMarkdownLossy.mockReturnValue('Edit after conflict')
    act(() => mocks.onEditorChange?.(mocks.editor))
    await act(async () => new Promise((resolve) => setTimeout(resolve, 600)))
    expect(onCommit).toHaveBeenCalledWith('Edit after conflict')
  })

  it('says nothing while the room is healthy and idle', async () => {
    const props = { value: 'Original', format: 'markdown' as const, onCommit: vi.fn(), liveTarget }
    const { queryByTestId, rerender } = render(<RecordBodyEditor {...props} />)
    await act(async () => Promise.resolve())
    expect(queryByTestId('data-editor-live-status')).toBeNull()

    connect()
    rerender(<RecordBodyEditor {...props} />)
    await act(async () => Promise.resolve())
    expect(queryByTestId('data-editor-live-status')).toBeNull()
  })
})
