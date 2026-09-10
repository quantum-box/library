import { describe, expect, it } from 'vitest'
import {
  organizationIdForUsername,
  organizationUsernameFromLocation,
  organizationUsernameFromPathname,
} from './organizationLocation'

describe('organizationUsernameFromPathname', () => {
  it('reads the organization out of a repository path', () => {
    expect(organizationUsernameFromPathname('/acme/handbook')).toBe('acme')
    expect(organizationUsernameFromPathname('/acme/handbook/data/rec-1')).toBe('acme')
    expect(organizationUsernameFromPathname('/acme/handbook/settings')).toBe('acme')
  })

  it('reads the organization out of the organization and legacy paths', () => {
    expect(organizationUsernameFromPathname('/organizations/acme')).toBe('acme')
    expect(organizationUsernameFromPathname('/repositories/acme/handbook')).toBe('acme')
  })

  it('decodes an escaped segment', () => {
    expect(organizationUsernameFromPathname('/organizations/quantum%20box')).toBe('quantum box')
  })

  it('returns null for the sections that own their first segment', () => {
    expect(organizationUsernameFromPathname('/')).toBeNull()
    expect(organizationUsernameFromPathname('/home')).toBeNull()
    expect(organizationUsernameFromPathname('/databases')).toBeNull()
    expect(organizationUsernameFromPathname('/databases/rec-1')).toBeNull()
    expect(organizationUsernameFromPathname('/repositories')).toBeNull()
    expect(organizationUsernameFromPathname('/chat')).toBeNull()
    expect(organizationUsernameFromPathname('/sync')).toBeNull()
    expect(organizationUsernameFromPathname('/public/acme/handbook')).toBeNull()
    expect(organizationUsernameFromPathname('/s/token')).toBeNull()
  })

  it('returns null for a lone segment, which matches no repository route', () => {
    expect(organizationUsernameFromPathname('/acme')).toBeNull()
  })
})

describe('organizationUsernameFromLocation', () => {
  it('falls back to the repository named by the data section search param', () => {
    expect(organizationUsernameFromLocation('/databases', 'acme/handbook')).toBe('acme')
    expect(organizationUsernameFromLocation('/databases/rec-1', 'acme/handbook')).toBe('acme')
  })

  it('prefers the path over the search param', () => {
    expect(organizationUsernameFromLocation('/beta/handbook', 'acme/handbook')).toBe('beta')
  })

  it('returns null when neither names a repository', () => {
    expect(organizationUsernameFromLocation('/databases', undefined)).toBeNull()
    expect(organizationUsernameFromLocation('/databases', 'handbook')).toBeNull()
  })
})

describe('organizationIdForUsername', () => {
  const organizations = [
    { id: 'org-1', username: 'Acme' },
    { id: 'org-2', username: 'beta' },
  ]
  const repositories = [{ operatorId: 'org-2', orgUsername: 'beta-inc' }]

  it('resolves through the repository that states the pairing', () => {
    expect(organizationIdForUsername('beta-inc', organizations, repositories)).toBe('org-2')
  })

  it('resolves through the organization name, ignoring case', () => {
    expect(organizationIdForUsername('acme', organizations, repositories)).toBe('org-1')
  })

  it('accepts an organization id, which older links carry', () => {
    expect(organizationIdForUsername('org-2', organizations, repositories)).toBe('org-2')
  })

  it('returns null for an unknown or empty username', () => {
    expect(organizationIdForUsername('gamma', organizations, repositories)).toBeNull()
    expect(organizationIdForUsername('', organizations, repositories)).toBeNull()
    expect(organizationIdForUsername(null, organizations, repositories)).toBeNull()
  })
})
