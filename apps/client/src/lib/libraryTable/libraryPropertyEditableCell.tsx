import { useCallback, useEffect, useRef, useState } from 'react'
import type { LibraryDataItem, LibraryProperty } from '../recordsApi'
import { getLibraryDataPropertyValue, propertyValueText } from './libraryPropertyFormat'
import { isInlineEditableProperty, parseEditablePropertyValue } from './libraryPropertyInput'
import { LibraryPropertyCell } from './libraryPropertyCells'

function EditableTextInput({
  value,
  inputType = 'text',
  onCommit,
  onCancel,
}: {
  value: string
  inputType?: 'text' | 'date'
  onCommit: (next: string) => void
  onCancel: () => void
}) {
  const [editValue, setEditValue] = useState(value)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    inputRef.current?.focus()
    inputRef.current?.select()
  }, [])

  const commit = useCallback(() => {
    onCommit(editValue)
  }, [editValue, onCommit])

  return (
    <input
      ref={inputRef}
      type={inputType}
      value={editValue}
      onChange={(event) => setEditValue(event.target.value)}
      onBlur={commit}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        if (event.key === 'Enter') commit()
        if (event.key === 'Escape') onCancel()
      }}
      className="w-full rounded border border-accent bg-canvas px-1 py-0.5 text-sm text-foreground outline-none"
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

  const displayText = currentValue ? propertyValueText(property, currentValue) ?? '' : ''

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
        value={displayText}
        inputType={property.typ === 'Date' ? 'date' : 'text'}
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
