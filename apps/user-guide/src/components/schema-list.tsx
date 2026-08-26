import type { SchemaField } from '@/lib/graphql-schema'
import { useState } from 'react'

/**
 * The schema carries far more fields than a reader wants at once, so the
 * list starts short and a filter is offered for finding a specific one.
 */
export function SchemaFieldList({
  fields,
  initialCount = 12,
}: {
  fields: SchemaField[]
  initialCount?: number
}) {
  const [filter, setFilter] = useState('')
  const [expanded, setExpanded] = useState(false)

  const needle = filter.trim().toLowerCase()
  const matched = needle
    ? fields.filter(field => field.name.toLowerCase().includes(needle))
    : fields
  const shown = expanded || needle ? matched : matched.slice(0, initialCount)
  const hidden = matched.length - shown.length

  return (
    <div className='space-y-3'>
      <input
        type='search'
        value={filter}
        onChange={event => setFilter(event.target.value)}
        placeholder='フィールド名で絞り込む'
        className='w-full rounded-md border border-slate-300 bg-white px-3 py-1.5 text-sm text-slate-900 placeholder:text-slate-400 focus:border-sky-500 focus:outline-none dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100'
      />

      {shown.length === 0 ? (
        <p className='text-sm text-slate-500'>一致するフィールドはありません。</p>
      ) : (
        <ul className='divide-y divide-slate-200 overflow-hidden rounded-lg border border-slate-200 dark:divide-slate-800 dark:border-slate-800'>
          {shown.map(field => (
            <li key={field.name} className='px-4 py-3'>
              <code className='font-mono text-[13px] text-slate-800 dark:text-slate-200'>
                <span className='font-semibold'>{field.name}</span>
                {field.args.length > 0 && `(${field.args.join(', ')})`}
                <span className='text-slate-500'>: {field.type}</span>
              </code>
              {field.description && (
                <p className='mt-1 text-sm text-slate-600 dark:text-slate-400'>
                  {field.description}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}

      {hidden > 0 && (
        <button
          type='button'
          onClick={() => setExpanded(true)}
          className='text-sm font-medium text-sky-700 hover:underline dark:text-sky-400'
        >
          残り {hidden} 件を表示
        </button>
      )}
    </div>
  )
}
