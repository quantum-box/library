import { Check, Copy } from 'lucide-react'
import { useCallback, useState, type ReactNode } from 'react'

export function Section({
  id,
  title,
  children,
}: {
  id?: string
  title: string
  children: ReactNode
}) {
  return (
    <section id={id} className='scroll-mt-24 space-y-4'>
      <h2 className='text-xl font-semibold tracking-tight text-slate-900 dark:text-slate-100'>
        {title}
      </h2>
      {children}
    </section>
  )
}

export function Prose({ children }: { children: ReactNode }) {
  return (
    <div className='space-y-3 text-[15px] leading-7 text-slate-700 dark:text-slate-300'>
      {children}
    </div>
  )
}

export function Callout({
  tone = 'note',
  title,
  children,
}: {
  tone?: 'note' | 'warning'
  title: string
  children: ReactNode
}) {
  const palette =
    tone === 'warning'
      ? 'border-amber-300 bg-amber-50 dark:border-amber-700/60 dark:bg-amber-950/30'
      : 'border-sky-300 bg-sky-50 dark:border-sky-700/60 dark:bg-sky-950/30'

  return (
    <div className={`rounded-lg border px-4 py-3 ${palette}`}>
      <p className='text-sm font-semibold text-slate-900 dark:text-slate-100'>
        {title}
      </p>
      <div className='mt-1 text-sm leading-6 text-slate-700 dark:text-slate-300'>
        {children}
      </div>
    </div>
  )
}

export function CodeBlock({
  code,
  language,
}: {
  code: string
  language?: string
}) {
  const [copied, setCopied] = useState(false)

  const copy = useCallback(async () => {
    await navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }, [code])

  return (
    <div className='group relative'>
      {language && (
        <span className='absolute left-3 top-2 text-[11px] font-medium uppercase tracking-wide text-slate-500'>
          {language}
        </span>
      )}
      <button
        type='button'
        onClick={copy}
        aria-label='コードをコピー'
        className='absolute right-2 top-2 rounded-md border border-slate-700 bg-slate-800 p-1.5 text-slate-300 opacity-0 transition hover:text-white focus:opacity-100 group-hover:opacity-100'
      >
        {copied ? (
          <Check className='h-3.5 w-3.5' />
        ) : (
          <Copy className='h-3.5 w-3.5' />
        )}
      </button>
      <pre
        className={`overflow-x-auto rounded-lg bg-slate-900 p-4 ${language ? 'pt-8' : ''}`}
      >
        <code className='font-mono text-[13px] leading-6 text-slate-100'>
          {code}
        </code>
      </pre>
    </div>
  )
}

export function Code({ children }: { children: ReactNode }) {
  return (
    <code className='rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[13px] text-slate-800 dark:bg-slate-800 dark:text-slate-200'>
      {children}
    </code>
  )
}

/**
 * Shown while something loaded from the API is on its way, and in place of
 * it when the call fails. The surrounding page is written to make sense
 * either way.
 */
export function LiveDataStatus({
  loading,
  error,
  loadingLabel,
}: {
  loading: boolean
  error?: string
  loadingLabel: string
}) {
  if (loading) {
    return (
      <p className='text-sm text-slate-500 dark:text-slate-400'>
        {loadingLabel}
      </p>
    )
  }
  if (error) {
    return (
      <div className='rounded-lg border border-slate-300 bg-slate-50 px-4 py-3 text-sm text-slate-600 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-400'>
        <p>{error}</p>
        <p className='mt-1'>
          この一覧は API から取得しています。API に到達できないときは表示できません。
        </p>
      </div>
    )
  }
  return null
}
