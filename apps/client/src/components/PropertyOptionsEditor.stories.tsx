import type { Meta, StoryObj } from '@storybook/react-vite'
import { useState } from 'react'
import { PropertyOptionsEditor } from './PropertyOptionsEditor'
import {
  optionDraftsFromProperty,
  type PropertyOptionDraft,
} from '../lib/propertyOptionDrafts'
import type { RepositoryPropertyDefinition } from '../lib/repositorySettingsApi'

function StatefulEditor({ property }: { property?: RepositoryPropertyDefinition }) {
  const [drafts, setDrafts] = useState<PropertyOptionDraft[]>(() =>
    optionDraftsFromProperty(property),
  )
  return (
    <div className="max-w-lg">
      <PropertyOptionsEditor drafts={drafts} onChange={setDrafts} />
    </div>
  )
}

const existingSelect: RepositoryPropertyDefinition = {
  id: 'property-status',
  name: 'Status',
  typ: 'SELECT',
  meta: {
    options: [
      { id: 'op_todo', key: 'todo', name: 'Todo' },
      { id: 'op_inProgress', key: 'inProgress', name: 'In progress' },
      { id: 'op_done', key: 'done', name: 'Done' },
    ],
  },
}

const meta = {
  title: 'Repositories/PropertyOptionsEditor',
  component: StatefulEditor,
} satisfies Meta<typeof StatefulEditor>

export default meta

type Story = StoryObj<typeof meta>

/** A brand new Select: one empty row, ready to type into. */
export const NewProperty: Story = {
  args: {},
}

/**
 * An existing Select. Its options are already referenced by records, so the
 * identifiers are locked and only the labels and the order can change.
 */
export const ExistingOptions: Story = {
  args: { property: existingSelect },
}
