import type { Operation, OperationGroup } from '@/lib/openapi'

const METHOD_STYLES: Record<string, string> = {
  get: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300',
  post: 'bg-sky-100 text-sky-800 dark:bg-sky-950 dark:text-sky-300',
  put: 'bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300',
  patch: 'bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300',
  delete: 'bg-rose-100 text-rose-800 dark:bg-rose-950 dark:text-rose-300',
}

export function EndpointGroups({ groups }: { groups: OperationGroup[] }) {
  return (
    <div className='space-y-8'>
      {groups.map(group => (
        <div key={group.name} className='space-y-2'>
          <h3 className='font-mono text-sm font-semibold text-slate-500 dark:text-slate-400'>
            {group.name}
          </h3>
          <ul className='divide-y divide-slate-200 overflow-hidden rounded-lg border border-slate-200 dark:divide-slate-800 dark:border-slate-800'>
            {group.operations.map(operation => (
              <EndpointRow
                key={`${operation.method}:${operation.path}`}
                operation={operation}
              />
            ))}
          </ul>
        </div>
      ))}
    </div>
  )
}

function EndpointRow({ operation }: { operation: Operation }) {
  const pathParams = operation.parameters.filter(p => p.location === 'path')
  const queryParams = operation.parameters.filter(p => p.location === 'query')

  return (
    <li className='flex flex-wrap items-baseline gap-x-3 gap-y-1 px-4 py-3'>
      <span
        className={`shrink-0 rounded px-1.5 py-0.5 font-mono text-[11px] font-bold uppercase ${
          METHOD_STYLES[operation.method] ?? 'bg-slate-100 text-slate-700'
        }`}
      >
        {operation.method}
      </span>
      <code className='font-mono text-[13px] text-slate-800 dark:text-slate-200'>
        {operation.path}
      </code>
      {operation.deprecated && (
        <span className='rounded bg-rose-100 px-1.5 py-0.5 text-[11px] font-medium text-rose-700 dark:bg-rose-950 dark:text-rose-300'>
          非推奨
        </span>
      )}
      {operation.summary && (
        <span className='w-full text-sm text-slate-600 dark:text-slate-400'>
          {operation.summary}
        </span>
      )}
      {(pathParams.length > 0 || queryParams.length > 0) && (
        <span className='w-full text-[13px] text-slate-500 dark:text-slate-500'>
          {pathParams.length > 0 && (
            <>パス: {pathParams.map(p => p.name).join(', ')}</>
          )}
          {pathParams.length > 0 && queryParams.length > 0 && ' / '}
          {queryParams.length > 0 && (
            <>
              クエリ:{' '}
              {queryParams
                .map(p => (p.required ? `${p.name}*` : p.name))
                .join(', ')}
            </>
          )}
        </span>
      )}
    </li>
  )
}
