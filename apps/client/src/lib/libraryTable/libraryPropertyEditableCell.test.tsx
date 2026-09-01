import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { LibraryPropertyEditableCell } from './libraryPropertyEditableCell'
import { libraryPropertyValueToGraphqlInput } from './libraryPropertyInput'
import type { LibraryDataItem, LibraryProperty } from '../recordsApi'

const dateProperty: LibraryProperty = {
  id: 'prop-date',
  name: 'date',
  typ: 'Date',
}

const booleanProperty: LibraryProperty = {
  id: 'prop-done',
  name: 'done',
  typ: 'Boolean',
}

const item: LibraryDataItem = {
  id: 'data-1',
  name: 'Sample row',
  propertyData: [{ propertyId: 'prop-date', value: { date: '2026-08-30' } }],
}

describe('LibraryPropertyEditableCell', () => {
  it('opens the editor on a single click where the surface asks for it', () => {
    render(
      <LibraryPropertyEditableCell
        item={item}
        property={dateProperty}
        activation="single"
        onCommit={vi.fn()}
      />,
    )

    fireEvent.click(screen.getByTestId('library-editable-cell-prop-date'))
    expect(screen.getByDisplayValue('2026-08-30')).toBeInTheDocument()
  })

  it('leaves a table cell on double-click activation', () => {
    render(
      <LibraryPropertyEditableCell item={item} property={dateProperty} onCommit={vi.fn()} />,
    )
    const cell = screen.getByTestId('library-editable-cell-prop-date')

    fireEvent.click(cell)
    expect(screen.queryByDisplayValue('2026-08-30')).not.toBeInTheDocument()

    fireEvent.doubleClick(cell)
    expect(screen.getByDisplayValue('2026-08-30')).toBeInTheDocument()
  })

  it('commits an emptied value as an explicit clear, because updateData patches', () => {
    const onCommit = vi.fn()
    render(
      <LibraryPropertyEditableCell
        item={item}
        property={dateProperty}
        activation="single"
        onCommit={onCommit}
      />,
    )

    fireEvent.click(screen.getByTestId('library-editable-cell-prop-date'))
    const input = screen.getByDisplayValue('2026-08-30')
    fireEvent.change(input, { target: { value: '' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    const next = onCommit.mock.calls[0][0] as LibraryDataItem
    expect(next.propertyData).toEqual([{ propertyId: 'prop-date', value: { date: '' } }])
    expect(libraryPropertyValueToGraphqlInput(dateProperty, next.propertyData[0].value)).toEqual({
      date: '',
    })
  })

  it('toggles a Boolean straight from the cell, with no edit mode in between', () => {
    const onCommit = vi.fn()
    render(
      <LibraryPropertyEditableCell
        item={{ id: 'data-1', name: 'Sample row', propertyData: [] }}
        property={booleanProperty}
        onCommit={onCommit}
      />,
    )

    const checkbox = screen.getByTestId('library-editable-input-prop-done')
    expect(checkbox).not.toBeChecked()
    fireEvent.click(checkbox)

    const next = onCommit.mock.calls[0][0] as LibraryDataItem
    expect(next.propertyData).toEqual([{ propertyId: 'prop-done', value: { boolean: true } }])
    expect(libraryPropertyValueToGraphqlInput(booleanProperty, next.propertyData[0].value)).toEqual(
      { boolean: true },
    )
  })

  it('keeps false as a value rather than reading it as an empty cell', () => {
    const onCommit = vi.fn()
    render(
      <LibraryPropertyEditableCell
        item={{
          id: 'data-1',
          name: 'Sample row',
          propertyData: [{ propertyId: 'prop-done', value: { boolean: true } }],
        }}
        property={booleanProperty}
        onCommit={onCommit}
      />,
    )

    fireEvent.click(screen.getByTestId('library-editable-input-prop-done'))

    const next = onCommit.mock.calls[0][0] as LibraryDataItem
    expect(next.propertyData).toEqual([{ propertyId: 'prop-done', value: { boolean: false } }])
    expect(libraryPropertyValueToGraphqlInput(booleanProperty, next.propertyData[0].value)).toEqual(
      { boolean: false },
    )
  })
})
