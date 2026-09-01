import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useState } from 'react'
import { PropertyOptionsEditor } from './PropertyOptionsEditor'
import {
  deriveOptionIdentifier,
  optionDraftsFromProperty,
  optionDraftsToPayload,
  type PropertyOptionDraft,
} from '../lib/propertyOptionDrafts'

function Harness({ onChange }: { onChange?: (drafts: PropertyOptionDraft[]) => void }) {
  const [drafts, setDrafts] = useState(() => optionDraftsFromProperty())
  return (
    <PropertyOptionsEditor
      drafts={drafts}
      onChange={(next) => {
        setDrafts(next)
        onChange?.(next)
      }}
    />
  )
}

describe('deriveOptionIdentifier', () => {
  it('camelCases a label into an identifier', () => {
    expect(deriveOptionIdentifier('In progress')).toBe('inProgress')
    expect(deriveOptionIdentifier('Done')).toBe('done')
    expect(deriveOptionIdentifier('Needs QA review')).toBe('needsQaReview')
  })

  it('derives nothing rather than inventing one', () => {
    // A label with no ASCII to work with, and one that would start with a
    // digit: both have to be typed rather than guessed.
    expect(deriveOptionIdentifier('進行中')).toBe('')
    expect(deriveOptionIdentifier('1st pass')).toBe('')
  })
})

describe('optionDraftsToPayload', () => {
  const draft = (over: Partial<PropertyOptionDraft>): PropertyOptionDraft => ({
    rowKey: 'row',
    identifier: '',
    label: '',
    identifierEdited: false,
    ...over,
  })

  it('fills the identifier from the label and drops the blank trailing row', () => {
    expect(
      optionDraftsToPayload([
        draft({ rowKey: 'a', label: 'In progress' }),
        draft({ rowKey: 'b' }),
      ]),
    ).toEqual([{ identifier: 'inProgress', label: 'In progress' }])
  })

  it('keeps the server-issued id of an option that already exists', () => {
    expect(
      optionDraftsToPayload([
        draft({ rowKey: 'a', id: 'op_todo', identifier: 'todo', label: 'To do', identifierEdited: true }),
      ]),
    ).toEqual([{ id: 'op_todo', identifier: 'todo', label: 'To do' }])
  })

  it('refuses a label whose identifier cannot be derived', () => {
    expect(() =>
      optionDraftsToPayload([draft({ rowKey: 'a', label: '進行中' })]),
    ).toThrow(/進行中/)
  })

  it('refuses duplicate identifiers', () => {
    expect(() =>
      optionDraftsToPayload([
        draft({ rowKey: 'a', label: 'Done' }),
        draft({ rowKey: 'b', label: 'done' }),
      ]),
    ).toThrow()
  })
})

describe('PropertyOptionsEditor', () => {
  it('adds a row and derives its identifier while nobody has typed one', () => {
    const onChange = vi.fn()
    render(<Harness onChange={onChange} />)

    fireEvent.change(screen.getByLabelText('Label of option 1'), {
      target: { value: 'In progress' },
    })
    expect(screen.getByLabelText('Identifier of option 1')).toHaveValue('inProgress')

    fireEvent.click(screen.getByRole('button', { name: 'Add option' }))
    fireEvent.change(screen.getByLabelText('Label of option 2'), {
      target: { value: 'Done' },
    })
    expect(screen.getByLabelText('Identifier of option 2')).toHaveValue('done')
  })

  it('stops deriving once the identifier is typed', () => {
    render(<Harness />)

    fireEvent.change(screen.getByLabelText('Identifier of option 1'), {
      target: { value: 'wip' },
    })
    fireEvent.change(screen.getByLabelText('Label of option 1'), {
      target: { value: 'In progress' },
    })

    expect(screen.getByLabelText('Identifier of option 1')).toHaveValue('wip')
  })

  it('turns a multi-line paste into one row per line', () => {
    render(<Harness />)

    fireEvent.paste(screen.getByLabelText('Label of option 1'), {
      clipboardData: { getData: () => 'todo = Todo\nIn progress\nDone' },
    })

    expect(screen.getByLabelText('Identifier of option 1')).toHaveValue('todo')
    expect(screen.getByLabelText('Label of option 2')).toHaveValue('In progress')
    expect(screen.getByLabelText('Identifier of option 3')).toHaveValue('done')
  })

  it('reorders rows without touching their values', () => {
    render(<Harness />)

    fireEvent.change(screen.getByLabelText('Label of option 1'), {
      target: { value: 'First' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Add option' }))
    fireEvent.change(screen.getByLabelText('Label of option 2'), {
      target: { value: 'Second' },
    })

    fireEvent.click(screen.getByRole('button', { name: 'Move option 2 up' }))

    expect(screen.getByLabelText('Label of option 1')).toHaveValue('Second')
    expect(screen.getByLabelText('Label of option 2')).toHaveValue('First')
  })
})
