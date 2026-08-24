import { useCallback, useEffect, useRef, useState } from 'react'
import type { LibraryDataItem, LibraryProperty } from '../recordsApi'
import { getLibraryDataPropertyValue, propertyValueEditText } from './libraryPropertyFormat'
import {
  isInlineEditableProperty,
  isMultilineEditableProperty,
  parseEditablePropertyValue,
} from './libraryPropertyInput'
import { LibraryPropertyCell } from './libraryPropertyCells'

const editableFieldClassName =
  'w-full rounded border border-accent bg-canvas px-1 py-0.5 text-sm text-foreground outline-none'

function EditableTextInput({
  value,
  inputType = 'text',
  multiline = false,
  onCommit,
  onCancel,
}: {
  value: string
  inputType?: 'text' | 'date'
  multiline?: boolean
  onCommit: (next: string) => void
  onCancel: () => void
}) {
  const [editValue, setEditValue] = useState(value)
  const fieldRef = useRef<HTMLInputElement | HTMLTextAreaElement | null>(null)

  useEffect(() => {
    fieldRef.current?.focus()
    fieldRef.current?.select()
  }, [])

  const commit = useCallback(() => {
    onCommit(editValue)
  }, [editValue, onCommit])

  if (multiline) {
    return (
      <textarea
        ref={(element) => {
          fieldRef.current = element
        }}
        value={editValue}
        rows={Math.min(12, Math.max(2, editValue.split('\n').length))}
        onChange={(event) => setEditValue(event.target.value)}
        onBlur={commit}
        onClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          // Enter has to insert a newline here, so committing needs a modifier.
          if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
            event.preventDefault()
            commit()
          }
          if (event.key === 'Escape') onCancel()
        }}
        className={`${editableFieldClassName} resize-y`}
      />
    )
  }

  return (
    <input
      ref={(element) => {
        fieldRef.current = element
      }}
      type={inputType}
      value={editValue}
      onChange={(event) => setEditValue(event.target.value)}
      onBlur={commit}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        if (event.key === 'Enter') commit()
        if (event.key === 'Escape') onCancel()
      }}
      className={editableFieldClassName}
    />
  )
}

function EditableSelect({
  property,
  value,
  onCommit,
  onCancel,
}: {
  property: LibraryProperty
  value: string
  onCommit: (optionId: string) => void
  onCancel: () => void
}) {
  const selectRef = useRef<HTMLSelectElement>(null)

  useEffect(() => {
    selectRef.current?.focus()
  }, [])

  return (
    <select
      ref={selectRef}
      defaultValue={value}
      className="w-full rounded border border-accent bg-canvas px-1 py-0.5 text-sm text-foreground outline-none"
      onClick={(event) => event.stopPropagation()}
      onBlur={onCancel}
      onChange={(event) => {
        onCommit(event.target.value)
      }}
    >
      <option value="">—</option>
      {(property.meta?.options ?? []).map((option) => (
        <option key={option.id} value={option.id}>
          {option.name ?? option.key ?? option.id}
        </option>
      ))}
    </select>
  )
}

export function LibraryPropertyEditableCell({
  item,
  property,
  disabled,
  onCommit,
}: {
  item: LibraryDataItem
  property: LibraryProperty
  disabled?: boolean
  onCommit: (item: LibraryDataItem) => void
}) {
  const [editing, setEditing] = useState(false)
  const currentValue = getLibraryDataPropertyValue(item, property.id)

  if (!isInlineEditableProperty(property)) {
    return <LibraryPropertyCell item={item} property={property} />
  }

  const editText = currentValue ? propertyValueEditText(property, currentValue) ?? '' : ''

  const handleCommitRaw = (raw: string) => {
    setEditing(false)
    const parsed = parseEditablePropertyValue(property, raw)
    if (!parsed) {
      onCommit({
        ...item,
        propertyData: item.propertyData.filter((entry) => entry.propertyId !== property.id),
      })
      return
    }
    const exists = item.propertyData.some((entry) => entry.propertyId === property.id)
    onCommit({
      ...item,
      propertyData: exists
        ? item.propertyData.map((entry) =>
            entry.propertyId === property.id ? { propertyId: property.id, value: parsed } : entry
          )
        : [...item.propertyData, { propertyId: property.id, value: parsed }],
    })
  }

  if (editing) {
    if (property.typ === 'Select') {
      return (
        <EditableSelect
          property={property}
          value={currentValue?.optionId ?? ''}
          onCommit={(optionId) => {
            setEditing(false)
            if (!optionId) {
              onCommit({
                ...item,
                propertyData: item.propertyData.filter(
                  (entry) => entry.propertyId !== property.id
                ),
              })
              return
            }
            handleCommitRaw(optionId)
          }}
          onCancel={() => setEditing(false)}
        />
      )
    }

    return (
      <EditableTextInput
        value={editText}
        inputType={property.typ === 'Date' ? 'date' : 'text'}
        multiline={isMultilineEditableProperty(property, editText)}
        onCommit={handleCommitRaw}
        onCancel={() => setEditing(false)}
      />
    )
  }

  return (
    <div
      className={`min-w-0 px-1 ${disabled ? '' : 'cursor-text'}`}
      data-testid={`library-editable-cell-${property.id}`}
      title={disabled ? undefined : 'Double-click to edit'}
      onDoubleClick={(event) => {
        if (disabled) return
        event.stopPropagation()
        setEditing(true)
      }}
    >
      <LibraryPropertyCell item={item} property={property} />
    </div>
  )
}

export function LibraryNameEditableCell({
  item,
  disabled,
  onCommit,
}: {
  item: LibraryDataItem
  disabled?: boolean
  onCommit: (name: string) => void
}) {
  const [editing, setEditing] = useState(false)

  if (editing) {
    return (
      <EditableTextInput
        value={item.name}
        onCommit={(next) => {
          setEditing(false)
          onCommit(next.trim() || item.name)
        }}
        onCancel={() => setEditing(false)}
      />
    )
  }

  return (
    <span
      className={`block truncate text-sm font-medium text-foreground ${disabled ? '' : 'cursor-text'}`}
      title={disabled ? undefined : 'Double-click to edit name'}
      onDoubleClick={(event) => {
        if (disabled) return
        event.stopPropagation()
        setEditing(true)
      }}
    >
      {item.name}
    </span>
  )
}
