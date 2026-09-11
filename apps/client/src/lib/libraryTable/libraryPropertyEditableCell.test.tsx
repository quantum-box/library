import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { LibraryPropertyEditableCell } from './libraryPropertyEditableCell'
import { libraryPropertyValueToGraphqlInput } from './libraryPropertyInput'
import type { LibraryDataItem, LibraryProperty } from '../recordsApi'
import type { LibraryRelationRecordLoader } from './relationRecords'

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

  it('selects and clears Relation records without entering data ids', async () => {
    const relationProperty: LibraryProperty = {
      id: 'prop-related',
      name: 'Related people',
      typ: 'Relation',
      meta: { databaseId: 'database-target' },
    }
    const relationLoader: LibraryRelationRecordLoader = {
      loadSelected: vi.fn().mockResolvedValue([{ id: 'data-1', name: 'Aoi Example' }]),
      loadPage: vi.fn()
        .mockResolvedValueOnce({
          items: [{ id: 'data-1', name: 'Aoi Example' }],
          hasMore: true,
          nextPage: 2,
          repositoryLabel: 'example / People',
        })
        .mockResolvedValueOnce({
          items: [{ id: 'data-2', name: 'Haru Example' }],
          hasMore: false,
          repositoryLabel: 'example / People',
        }),
    }
    const onCommit = vi.fn()
    render(
      <LibraryPropertyEditableCell
        item={{
          id: 'data-source',
          name: 'Source',
          propertyData: [{ propertyId: relationProperty.id, value: { dataIds: ['data-1'] } }],
        }}
        property={relationProperty}
        activation="single"
        relationLoader={relationLoader}
        onCommit={onCommit}
      />,
    )

    await waitFor(() => expect(screen.getByText('Aoi Example')).toBeInTheDocument())
    fireEvent.click(screen.getByTestId('library-relation-cell-prop-related'))
    const options = await screen.findByTestId('library-relation-options')
    fireEvent.click(await screen.findByRole('button', { name: 'Load more records' }))
    await waitFor(() => expect(within(options).getByText('Haru Example')).toBeInTheDocument())
    expect(relationLoader.loadPage).toHaveBeenLastCalledWith('database-target', 2)
    fireEvent.click(within(options).getByText('Aoi Example'))
    fireEvent.click(within(options).getByText('Haru Example'))
    fireEvent.click(screen.getByRole('button', { name: 'Save 1 links' }))

    const next = onCommit.mock.calls[0][0] as LibraryDataItem
    expect(next.propertyData).toEqual([
      { propertyId: relationProperty.id, value: { dataIds: ['data-2'] } },
    ])
  })
})
