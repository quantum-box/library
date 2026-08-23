import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { BlockNoteEditor } from '@blocknote/core'
import { BLOCKNOTE_FIXTURE_CASES } from './blocknoteFixtureCases'

/**
 * Generates and guards the BlockNote documents that the Rust markdown
 * serializer is tested against.
 *
 * Run `npm run fixtures:blocknote` to regenerate after changing a case or
 * upgrading BlockNote, then review the diff. Left alone, this test is the
 * drift detector: if an upgrade renames a prop or changes a default, it
 * fails here with a readable diff instead of silently changing what the API
 * renders to Markdown.
 */
const FIXTURE_DIR = join(
  dirname(fileURLToPath(import.meta.url)),
  '../../../../packages/blocknote/tests/fixtures',
)

/**
 * BlockNote assigns a random ULID to every block. Without this the fixtures
 * would churn on every run and the drift check could never pass twice.
 */
function withStableIds(blocks: unknown[], counter = { next: 1 }): unknown[] {
  return blocks.map((block) => {
    const record = block as Record<string, unknown>
    const stable: Record<string, unknown> = { ...record, id: `block-${counter.next++}` }
    if (Array.isArray(record.children)) {
      stable.children = withStableIds(record.children, counter)
    }
    return stable
  })
}

function documentFor(blocks: unknown[]): unknown[] {
  const editor = BlockNoteEditor.create()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  editor.replaceBlocks(editor.document, blocks as any)
  return withStableIds(editor.document as unknown[])
}

const shouldWrite = process.env.UPDATE_FIXTURES === '1'

describe('blocknote fixtures', () => {
  it.each(BLOCKNOTE_FIXTURE_CASES.map((testCase) => [testCase.name, testCase] as const))(
    '%s matches the committed document',
    (name, testCase) => {
      const document = documentFor(testCase.blocks)
      const path = join(FIXTURE_DIR, `${name}.json`)

      if (shouldWrite) {
        mkdirSync(FIXTURE_DIR, { recursive: true })
        writeFileSync(path, `${JSON.stringify(document, null, 2)}\n`)
        return
      }

      expect(
        existsSync(path),
        `${path} is missing. Run "npm run fixtures:blocknote".`,
      ).toBe(true)
      expect(
        document,
        `${name} drifted from the committed fixture. Run "npm run fixtures:blocknote" and review the diff.`,
      ).toEqual(JSON.parse(readFileSync(path, 'utf8')))
    },
  )
})
