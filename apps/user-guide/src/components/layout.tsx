import { apiBaseUrl } from '@/lib/config'
import { BookOpen, ExternalLink } from 'lucide-react'
import { NavLink, Outlet } from 'react-router-dom'

const NAV = [
  { to: '/', label: 'はじめに', end: true },
  { to: '/api-keys', label: 'API キーの発行と失効' },
  { to: '/rest', label: 'REST API' },
  { to: '/graphql', label: 'GraphQL API' },
]

const EXTERNAL = [
  { href: `${apiBaseUrl}/v1beta/swagger-ui`, label: 'Swagger UI' },
  { href: `${apiBaseUrl}/v1beta/redoc`, label: 'ReDoc' },
  { href: `${apiBaseUrl}/v1/graphql`, label: 'GraphQL Playground' },
]

export function Layout() {
  return (
    <div className='min-h-screen bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100'>
      <header className='sticky top-0 z-10 border-b border-slate-200 bg-white/90 backdrop-blur dark:border-slate-800 dark:bg-slate-950/90'>
        <div className='mx-auto flex max-w-5xl items-center gap-3 px-6 py-4'>
          <BookOpen className='h-5 w-5 text-sky-600 dark:text-sky-400' />
          <span className='font-semibold tracking-tight'>
            Library ユーザーガイド
          </span>
          <span className='rounded-full border border-slate-300 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:border-slate-700 dark:text-slate-400'>
            Beta
          </span>
        </div>
      </header>

      <div className='mx-auto flex max-w-5xl gap-10 px-6 py-10'>
        <nav className='hidden w-52 shrink-0 lg:block'>
          <div className='sticky top-24 space-y-6'>
            <ul className='space-y-1'>
              {NAV.map(item => (
                <li key={item.to}>
                  <NavLink
                    to={item.to}
                    end={item.end}
                    className={({ isActive }) =>
                      `block rounded-md px-3 py-1.5 text-sm transition ${
                        isActive
                          ? 'bg-sky-50 font-medium text-sky-700 dark:bg-sky-950/50 dark:text-sky-300'
                          : 'text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-900'
                      }`
                    }
                  >
                    {item.label}
                  </NavLink>
                </li>
              ))}
            </ul>

            <div className='space-y-1 border-t border-slate-200 pt-4 dark:border-slate-800'>
              <p className='px-3 pb-1 text-[11px] font-semibold uppercase tracking-wide text-slate-400'>
                リファレンス
              </p>
              {EXTERNAL.map(item => (
                <a
                  key={item.href}
                  href={item.href}
                  target='_blank'
                  rel='noopener noreferrer'
                  className='flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm text-slate-600 transition hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-900'
                >
                  {item.label}
                  <ExternalLink className='h-3 w-3 text-slate-400' />
                </a>
              ))}
            </div>
          </div>
        </nav>

        <main className='min-w-0 flex-1 space-y-10 pb-20'>
          <Outlet />
        </main>
      </div>
    </div>
  )
}
