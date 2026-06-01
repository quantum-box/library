import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { LibraryPropertyCell } from './libraryPropertyCells'
import type { LibraryDataItem, LibraryProperty } from '../recordsApi'

const statusProperty: LibraryProperty = {
  id: 'prop-status',
  name: 'Status',
  typ: 'Select',
  meta: {
    options: [{ id: 'opt-1', key: 'todo', name: 'Todo' }],
  },
}

const item: LibraryDataItem = {
  id: 'data-1',
  name: 'Sample row',
  propertyData: [
    {
      propertyId: 'prop-status',
      value: { optionId: 'opt-1' },
    },
    {
      propertyId: 'prop-body',
      value: { string: 'Hello world' },
    },
  ],
}

describe('LibraryPropertyCell', () => {
  it('renders Select options using property metadata', () => {
    render(<LibraryPropertyCell item={item} property={statusProperty} />)
    expect(screen.getByText('Todo')).toBeInTheDocument()
  })

  it('renders plain string values', () => {
    render(
      <LibraryPropertyCell
        item={item}
        property={{ id: 'prop-body', name: 'Body', typ: 'String' }}
      />
    )
    expect(screen.getByText('Hello world')).toBeInTheDocument()
  })
})
