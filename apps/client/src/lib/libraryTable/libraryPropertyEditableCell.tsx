import { useCallback, useEffect, useRef, useState } from 'react'
import type { LibraryDataItem, LibraryProperty } from '../recordsApi'
import { getLibraryDataPropertyValue, propertyValueEditText } from './libraryPropertyFormat'
import {
  clearedPropertyValue,
  isEmptyPropertyValue,
  isInlineEditableProperty,
  isMultilineEditableProperty,
  mergeLibraryDataProperty,
  parseEditablePropertyValue,
} from './libraryPropertyInput'
import { LibraryPropertyCell } from './libraryPropertyCells'
import { t } from '../../i18n'

const editableFieldClassName =
  'w-full rounded border border-accent bg-canvas px-1 py-0.5 text-sm text-foreground outline-none'

function EditableTextInput({
  value,
  inputType = 'text',
  multiline = false,
  testId,
  onCommit,
  onCancel,
}: {
  value: string
  inputType?: 'text' | 'date'
  multiline?: boolean
  testId?: string
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
        data-testid={testId}
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
      data-testid={testId}
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
  testId,
  onCommit,
  onCancel,
}: {
  property: LibraryProperty
  value: string
  testId?: string
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
      data-testid={testId}
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
  activation = 'double',
  onCommit,
}: {
  item: LibraryDataItem
  property: LibraryProperty
  disabled?: boolean
  /**
   * How the editor opens. A table row uses `double` so a click can still
   * select the row; a property list has nothing else to click, so it uses
   * `single` -- a value nobody can find how to edit reads as read-only.
   */
  activation?: 'single' | 'double'
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
    // An emptied field has to keep travelling as an explicit empty value:
    // updateData patches, so dropping the entry would leave the old value on
    // the server while the screen showed the field as cleared.
    const parsed = parseEditablePropertyValue(property, raw) ?? clearedPropertyValue(property)
    if (!parsed) {
      onCommit({
        ...item,
        propertyData: item.propertyData.filter((entry) => entry.propertyId !== property.id),
      })
      return
    }
    onCommit(mergeLibraryDataProperty(item, property.id, parsed))
  }

  if (editing) {
    if (property.typ === 'Select') {
      return (
        <EditableSelect
          property={property}
          value={currentValue?.optionId ?? ''}
          testId={`library-editable-input-${property.id}`}
          onCommit={(optionId) => handleCommitRaw(optionId)}
          onCancel={() => setEditing(false)}
        />
      )
    }

    return (
      <EditableTextInput
        value={editText}
        testId={`library-editable-input-${property.id}`}
        inputType={property.typ === 'Date' ? 'date' : 'text'}
        multiline={isMultilineEditableProperty(property, editText)}
        onCommit={handleCommitRaw}
        onCancel={() => setEditing(false)}
      />
    )
  }

  const singleClick = activation === 'single'
  const beginEditing = (event: { stopPropagation: () => void }) => {
    if (disabled) return
    event.stopPropagation()
    setEditing(true)
  }

  return (
    <div
      className={`min-w-0 rounded px-1 ${disabled ? '' : 'cursor-text'} ${singleClick ? 'hover:bg-muted/60' : ''}`}
      data-testid={`library-editable-cell-${property.id}`}
      title={disabled
        ? undefined
        : singleClick
          ? t('common.clickToEdit')
          : t('table.doubleClickToEdit')}
      role={singleClick && !disabled ? 'button' : undefined}
      tabIndex={singleClick && !disabled ? 0 : undefined}
      aria-label={singleClick && !disabled
        ? t('common.editNamed', { name: property.name })
        : undefined}
      onClick={singleClick ? beginEditing : undefined}
      onKeyDown={singleClick ? (event) => {
        if (disabled) return
        if (event.key !== 'Enter' && event.key !== ' ') return
        event.preventDefault()
        beginEditing(event)
      } : undefined}
      onDoubleClick={singleClick ? undefined : beginEditing}
    >
      {singleClick && (!currentValue || isEmptyPropertyValue(currentValue)) ? (
        <span className="block text-sm text-subtle-foreground">
          {t('libraryTable.emptyValue')}
        </span>
      ) : (
        <LibraryPropertyCell item={item} property={property} />
      )}
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
      title={disabled ? undefined : t('libraryTable.doubleClickToEditName')}
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
