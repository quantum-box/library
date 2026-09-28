import { Badge, Button, Input, Label } from '@tachyon-sdk/native-ui'
import { Loader2, Plus, RefreshCw, Shield, Trash2, UsersRound } from 'lucide-react'
import { type FormEvent, useCallback, useEffect, useRef, useState } from 'react'
import { useI18n } from '../i18n'
import { loadStoredAuthIdentity } from '../lib/auth'
import {
  changeRepositoryMemberRole,
  fetchRepositoryMembers,
  inviteRepositoryMember,
  removeRepositoryMember,
  type RepositoryMember,
  type RepositoryMemberRole,
  type RepositorySettingsTarget,
} from '../lib/repositorySettingsApi'

const roles: RepositoryMemberRole[] = ['reader', 'writer', 'owner']

function roleForMember(member: RepositoryMember): RepositoryMemberRole {
  const policy = `${member.policyName ?? ''} ${member.policyId}`.toLowerCase()
  if (policy.includes('owner') || policy.includes('admin')) return 'owner'
  if (policy.includes('writer') || policy.includes('member')) return 'writer'
  return 'reader'
}

function memberName(member: RepositoryMember): string {
  return member.user?.name?.trim() || member.user?.email?.trim() || member.userId
}

export function RepositoryAccessSection({
  target,
  repositoryId,
}: {
  target: RepositorySettingsTarget
  repositoryId: string
}) {
  const { t } = useI18n()
  const loadRevision = useRef(0)
  const [expanded, setExpanded] = useState(false)
  const [members, setMembers] = useState<RepositoryMember[]>([])
  const [loading, setLoading] = useState(false)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)
  const [username, setUsername] = useState('')
  const [role, setRole] = useState<RepositoryMemberRole>('reader')
  const [busy, setBusy] = useState(false)
  const actorId = loadStoredAuthIdentity()?.userId

  const loadMembers = useCallback(async () => {
    const revision = ++loadRevision.current
    setLoading(true)
    setLoadError(null)
    try {
      const next = await fetchRepositoryMembers(target)
      if (revision === loadRevision.current) setMembers(next)
    } catch (error) {
      if (revision === loadRevision.current) {
        setLoadError(error instanceof Error ? error.message : t('repoAccess.loadFailed'))
        setMembers([])
      }
    } finally {
      if (revision === loadRevision.current) setLoading(false)
    }
  }, [target, t])

  useEffect(() => {
    if (!expanded) return
    void loadMembers()
    return () => {
      loadRevision.current += 1
    }
  }, [expanded, loadMembers])

  const canManage = Boolean(actorId && members.some(
    (member) => member.userId === actorId && roleForMember(member) === 'owner',
  ))

  const runMemberAction = async (action: () => Promise<void>) => {
    if (busy) return
    setBusy(true)
    setActionError(null)
    try {
      await action()
      await loadMembers()
    } catch (error) {
      setActionError(error instanceof Error ? error.message : t('repoAccess.actionFailed'))
    } finally {
      setBusy(false)
    }
  }

  const handleInvite = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy) return
    setBusy(true)
    setActionError(null)
    try {
      await inviteRepositoryMember({ target, repoId: repositoryId, username, role })
      setUsername('')
      await loadMembers()
    } catch (error) {
      setActionError(error instanceof Error ? error.message : t('repoAccess.actionFailed'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section
      className="mt-5 overflow-hidden rounded-lg border border-border bg-background shadow-soft"
      aria-labelledby="repository-access-heading"
      data-testid="repository-access-section"
    >
      <div className="flex flex-col gap-3 border-b border-border bg-surface px-4 py-3 sm:flex-row sm:items-center">
        <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
          <UsersRound className="size-4" aria-hidden="true" />
        </span>
        <div className="min-w-0 flex-1">
          <h2 id="repository-access-heading" className="text-sm font-semibold">
            {t('repoAccess.title')}
          </h2>
          <p className="text-2xs text-muted-foreground">{t('repoAccess.subtitle')}</p>
        </div>
        <Button
          type="button"
          variant="secondary"
          size="sm"
          aria-expanded={expanded}
          data-testid="repository-access-toggle"
          onClick={() => {
            setExpanded((current) => !current)
            setActionError(null)
          }}
        >
          <Shield aria-hidden="true" />
          {t(expanded ? 'repoAccess.hide' : 'repoAccess.manage')}
        </Button>
      </div>

      {expanded && (
        <div className="space-y-4 p-4">
          {loadError ? (
            <div role="alert" className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
              <span className="min-w-0 flex-1">{t('repoAccess.error', { message: loadError })}</span>
              <Button type="button" variant="ghost" size="sm" disabled={loading} onClick={() => void loadMembers()}>
                <RefreshCw className={loading ? 'animate-spin motion-reduce:animate-none' : ''} aria-hidden="true" />
                {t('common.retry')}
              </Button>
            </div>
          ) : null}

          {loading && members.length === 0 ? (
            <p className="flex items-center gap-2 text-xs text-muted-foreground" aria-busy="true">
              <Loader2 className="size-4 animate-spin motion-reduce:animate-none" aria-hidden="true" />
              {t('common.loading')}
            </p>
          ) : null}

          {!loadError && members.length === 0 && !loading ? (
            <p className="text-xs text-muted-foreground">{t('repoAccess.noMembers')}</p>
          ) : null}

          {members.length > 0 && (
            <ul className="divide-y divide-border rounded-md border border-border">
              {members.map((member) => {
                const name = memberName(member)
                const memberRole = roleForMember(member)
                const inherited = member.permissionSource === 'ORG'
                return (
                  <li key={`${member.userId}:${member.policyId}`} className="flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center">
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium">{name}</p>
                      {member.user?.email && member.user.name ? (
                        <p className="truncate text-2xs text-muted-foreground">{member.user.email}</p>
                      ) : null}
                      {inherited ? (
                        <p className="text-2xs text-muted-foreground">{t('repoAccess.organizationAccess')}</p>
                      ) : null}
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant={memberRole === 'owner' ? 'success' : 'neutral'}>
                        {t(`repoAccess.role.${memberRole}`)}
                      </Badge>
                      {canManage && !inherited ? (
                        <>
                          <label className="sr-only" htmlFor={`repo-member-role-${member.userId}`}>
                            {t('repoAccess.roleFor', { name })}
                          </label>
                          <select
                            id={`repo-member-role-${member.userId}`}
                            value={memberRole}
                            disabled={busy}
                            className="h-8 rounded-md border border-input bg-background px-2 text-xs text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50"
                            onChange={(event) => {
                              const nextRole = event.target.value as RepositoryMemberRole
                              if (nextRole === memberRole) return
                              void runMemberAction(() => changeRepositoryMemberRole({
                                target,
                                repoId: repositoryId,
                                userId: member.userId,
                                role: nextRole,
                              }))
                            }}
                          >
                            {roles.map((option) => (
                              <option key={option} value={option}>{t(`repoAccess.role.${option}`)}</option>
                            ))}
                          </select>
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon"
                            className="size-8 text-muted-foreground hover:text-destructive"
                            aria-label={t('repoAccess.removeMember', { name })}
                            disabled={busy}
                            onClick={() => void runMemberAction(() => removeRepositoryMember({
                              target,
                              repoId: repositoryId,
                              userId: member.userId,
                            }))}
                          >
                            <Trash2 aria-hidden="true" />
                          </Button>
                        </>
                      ) : null}
                    </div>
                  </li>
                )
              })}
            </ul>
          )}

          {canManage ? (
            <form onSubmit={(event) => void handleInvite(event)} className="grid gap-3 rounded-md border border-border bg-surface/60 p-3 sm:grid-cols-[minmax(0,1fr)_9rem_auto] sm:items-end">
              <div className="space-y-1.5">
                <Label htmlFor="repository-member-username">{t('repoAccess.username')}</Label>
                <Input
                  id="repository-member-username"
                  autoComplete="off"
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                  placeholder={t('repoAccess.usernamePlaceholder')}
                  disabled={busy}
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="repository-member-role">{t('repoAccess.permission')}</Label>
                <select
                  id="repository-member-role"
                  value={role}
                  disabled={busy}
                  className="h-9 w-full rounded-md border border-input bg-background px-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50"
                  onChange={(event) => setRole(event.target.value as RepositoryMemberRole)}
                >
                  {roles.map((option) => (
                    <option key={option} value={option}>{t(`repoAccess.role.${option}`)}</option>
                  ))}
                </select>
              </div>
              <Button type="submit" variant="primary" size="sm" disabled={busy || !username.trim()}>
                {busy
                  ? <Loader2 className="animate-spin motion-reduce:animate-none" aria-hidden="true" />
                  : <Plus aria-hidden="true" />}
                {t('repoAccess.addMember')}
              </Button>
              <p className="text-2xs text-muted-foreground sm:col-span-3">
                {t('repoAccess.usernameHint')}
              </p>
            </form>
          ) : !loading && !loadError ? (
            <p className="text-xs text-muted-foreground">{t('repoAccess.ownerOnly')}</p>
          ) : null}

          {actionError ? (
            <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
              {t('repoAccess.error', { message: actionError })}
            </p>
          ) : null}
        </div>
      )}
    </section>
  )
}
