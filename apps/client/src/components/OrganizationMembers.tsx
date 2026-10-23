import { Badge, Button, Input } from '@tachyon-sdk/native-ui'
import { RefreshCw, UsersRound } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { useI18n } from '../i18n'
import { collator } from '../i18n/format'
import { fetchOrganizationMembers, type OrganizationMember } from '../lib/recordsApi'

const roles = ['OWNER', 'MANAGER', 'GENERAL'] as const

export function OrganizationMembers({ username, operatorId }: { username: string; operatorId: string }) {
  const { t, formatCount, locale } = useI18n()
  const [members, setMembers] = useState<OrganizationMember[]>([])
  const [loading, setLoading] = useState(true)
  const [failed, setFailed] = useState(false)
  const [revision, setRevision] = useState(0)
  const [search, setSearch] = useState('')

  useEffect(() => {
    let active = true
    Promise.resolve().then(() => {
      if (!active) return []
      setLoading(true)
      setFailed(false)
      return fetchOrganizationMembers(username, operatorId)
    }).then((result) => {
      if (active) setMembers(result)
    }).catch(() => {
      if (active) setFailed(true)
    }).finally(() => {
      if (active) setLoading(false)
    })
    return () => { active = false }
  }, [username, operatorId, revision])

  const visibleMembers = useMemo(() => {
    const query = search.trim().toLocaleLowerCase()
    return members.filter((member) =>
      [member.name, member.email, member.id].some((value) => value?.toLocaleLowerCase().includes(query)),
    ).sort((a, b) => collator(locale).compare(a.name || a.email || a.id, b.name || b.email || b.id))
  }, [members, search, locale])

  return (
    <section id="members" className="border-b border-border" aria-labelledby="organization-members-heading" aria-busy={loading}>
      <div className="flex h-10 items-center gap-2 bg-surface/60 px-4 md:px-5">
        <UsersRound className="size-4 text-muted-foreground" aria-hidden="true" />
        <h2 id="organization-members-heading" className="text-sm font-semibold">{t('orgMembers.title')}</h2>
        {!loading && !failed && <Badge variant="neutral">{formatCount(members.length)}</Badge>}
        <Button className="ml-auto" variant="ghost" size="sm" disabled={loading} onClick={() => setRevision((value) => value + 1)} aria-label={t('orgMembers.refresh')}>
          <RefreshCw className={loading ? 'animate-spin' : ''} aria-hidden="true" />
        </Button>
      </div>
      <div className="space-y-4 border-t border-border p-4 md:p-5">
        <p className="text-xs leading-5 text-muted-foreground">{t('orgMembers.description')}</p>
        <div className="grid gap-3 sm:grid-cols-3">
          {roles.map((role) => (
            <div key={role} className="rounded-md border border-border p-3">
              <p className="text-xs font-semibold">{t(`orgMembers.${role}`)}</p>
              <p className="mt-1 text-xs leading-5 text-muted-foreground">{t(`orgMembers.${role}Hint`)}</p>
            </div>
          ))}
        </div>
        {loading ? <p role="status" className="text-sm text-muted-foreground">{t('orgMembers.loading')}</p> : failed ? (
          <div role="alert" className="flex items-center gap-3 text-sm">
            <p>{t('orgMembers.failed')}</p>
            <Button size="sm" onClick={() => setRevision((value) => value + 1)}>{t('common.tryAgain')}</Button>
          </div>
        ) : (
          <>
            <Input value={search} onChange={(event) => setSearch(event.target.value)} placeholder={t('orgMembers.search')} aria-label={t('orgMembers.search')} />
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm">
                <thead className="text-xs text-muted-foreground"><tr>
                  <th scope="col" className="py-2 font-medium">{t('orgMembers.member')}</th>
                  <th scope="col" className="py-2 font-medium">{t('orgMembers.role')}</th>
                </tr></thead>
                <tbody>{visibleMembers.map((member) => {
                  const knownRole = roles.find((role) => role === member.role.toUpperCase())
                  const name = member.name?.trim() || member.email || member.id
                  return (
                    <tr key={member.id} className="border-t border-border">
                      <td className="py-3 pr-4"><div className="flex items-center gap-3">
                        <span className="flex size-8 shrink-0 items-center justify-center overflow-hidden rounded-full bg-selected text-xs" aria-hidden="true">
                          {member.image ? <img src={member.image} alt="" className="size-8 object-cover" /> : name.slice(0, 1).toLocaleUpperCase()}
                        </span>
                        <div className="min-w-0"><p className="break-all font-medium">{name}</p>{member.email && member.email !== name && <p className="break-all text-xs text-muted-foreground">{member.email}</p>}</div>
                      </div></td>
                      <td className="py-3"><Badge variant="outline">{knownRole ? t(`orgMembers.${knownRole}`) : member.role}</Badge></td>
                    </tr>
                  )
                })}</tbody>
              </table>
              {visibleMembers.length === 0 && <p className="py-6 text-center text-sm text-muted-foreground">{t('orgMembers.empty')}</p>}
            </div>
          </>
        )}
      </div>
    </section>
  )
}
