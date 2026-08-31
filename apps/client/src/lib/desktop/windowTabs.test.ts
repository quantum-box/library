import { describe, expect, it } from 'vitest'
import { tabTitleForPath } from './windowTabs'

describe('tabTitleForPath', () => {
  it('names the workspace-level routes', () => {
    expect(tabTitleForPath('/')).toBe('Library')
    expect(tabTitleForPath('/home')).toBe('Home')
    expect(tabTitleForPath('/repositories')).toBe('Repositories')
    expect(tabTitleForPath('/databases')).toBe('All data')
    expect(tabTitleForPath('/chat')).toBe('Ask Library')
    expect(tabTitleForPath('/sync')).toBe('Sync status')
  })

  it('keeps the view name for the all-data boards', () => {
    expect(tabTitleForPath('/databases/board')).toBe('All data · Board')
    expect(tabTitleForPath('/databases/workflow')).toBe('All data · Workflow')
  })

  it('scopes repository routes to organization/repository', () => {
    expect(tabTitleForPath('/acme/handbook')).toBe('acme/handbook')
    expect(tabTitleForPath('/acme/handbook/data')).toBe('acme/handbook · Data')
    expect(tabTitleForPath('/acme/handbook/settings')).toBe('acme/handbook · Settings')
    expect(tabTitleForPath('/acme/handbook/api')).toBe('acme/handbook · API keys')
  })

  it('stays on the repository scope for a single record', () => {
    expect(tabTitleForPath('/acme/handbook/data/rec-1')).toBe('acme/handbook · Data')
    expect(tabTitleForPath('/databases/rec-1')).toBe('All data')
  })

  it('handles the legacy and public route prefixes', () => {
    expect(tabTitleForPath('/organizations/acme')).toBe('acme')
    expect(tabTitleForPath('/repositories/acme/handbook')).toBe('acme/handbook')
    expect(tabTitleForPath('/repositories/acme/handbook/settings')).toBe(
      'acme/handbook · Settings'
    )
    expect(tabTitleForPath('/public/acme/handbook')).toBe('acme/handbook · Public')
    expect(tabTitleForPath('/public/acme/handbook/rec-1')).toBe('acme/handbook · Public')
  })

  it('decodes percent-encoded path segments', () => {
    expect(tabTitleForPath('/acme/my%20repo')).toBe('acme/my repo')
  })
})
