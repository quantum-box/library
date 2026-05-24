import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@/auth'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Eye, GitBranch, Lock, ShieldAlert, Trash2, UserPlus, Users } from 'lucide-react'
import { type FormEvent, useCallback, useEffect, useMemo, useState } from 'react'
import type { GetRepoMembersQuery } from '@/gen/graphql'
import {
  changeLibraryMemberPolicyAction,
  getRepoMembersForSettingsAction,
  inviteLibraryMemberAction,
  removeLibraryMemberAction,
} from '@/app/v1beta/[org]/[repo]/settings/actions'
import {
  type DisplayRepoMember,
  type LibraryPolicy,
  getSafeMemberActionError,
  libraryPolicies,
  normalizeRepoMembers,
} from '@/app/v1beta/[org]/[repo]/settings/member-policy'

export const Route = createFileRoute('/v1beta/$org/$repo/settings')({
  component: SettingsPage,
})

type RepoSettingsData = GetRepoMembersQuery['repo'] & {
  name: string
  username: string
  description?: string | null
  isPublic: boolean
}

function SettingsPage() {
  const { org, repo } = Route.useParams()
  const { session } = useAuth()
  const [repoData, setRepoData] = useState<RepoSettingsData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [memberQuery, setMemberQuery] = useState('')
  const [inviteTarget, setInviteTarget] = useState('')
  const [invitePolicy, setInvitePolicy] = useState<LibraryPolicy>('library:member')
  const [pendingAction, setPendingAction] = useState<string | null>(null)

  const loadSettings = useCallback(async () => {
    if (session?.user && !session.user.accessToken) {
      setLoading(false)
      return
    }

    setLoading(true)
    setError(null)
    try {
      const result = await getRepoMembersForSettingsAction({
        orgUsername: org,
        repoUsername: repo,
      })
      setRepoData(result ?? null)
    } catch (err) {
      setRepoData(null)
      setError(getSafeMemberActionError(err))
    } finally {
      setLoading(false)
    }
  }, [org, repo, session?.user])

  useEffect(() => {
    void loadSettings()
  }, [loadSettings])

  const members = useMemo(
    () => normalizeRepoMembers(repoData?.members ?? []),
    [repoData?.members],
  )

  const filteredMembers = useMemo(() => {
    const query = memberQuery.trim().toLowerCase()
    if (!query) return members
    return members.filter((member) =>
      [
        member.displayName,
        member.email ?? '',
        member.userId,
        member.policyLabel,
      ].some((value) => value.toLowerCase().includes(query)),
    )
  }, [memberQuery, members])

  const runMemberAction = async (key: string, action: () => Promise<void>) => {
    setPendingAction(key)
    setError(null)
    setNotice(null)
    try {
      await action()
      await loadSettings()
    } catch (err) {
      setError(getSafeMemberActionError(err))
    } finally {
      setPendingAction(null)
    }
  }

  const handleInvite = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const usernameOrEmail = inviteTarget.trim()
    if (!repoData || !usernameOrEmail) return

    void runMemberAction('invite', async () => {
      await inviteLibraryMemberAction({
        orgUsername: org,
        repoUsername: repo,
        repoId: repoData.id,
        usernameOrEmail,
        policy: invitePolicy,
      })
      setInviteTarget('')
      setNotice(`${usernameOrEmail} に ${invitePolicy} を付与しました。`)
    })
  }

  const handlePolicyChange = (member: DisplayRepoMember, policy: LibraryPolicy) => {
    if (!repoData || member.policy === policy) return

    void runMemberAction(`change:${member.userId}`, async () => {
      await changeLibraryMemberPolicyAction({
        repoId: repoData.id,
        userId: member.userId,
        policy,
      })
      setNotice(`${member.displayName} の policy を ${policy} に変更しました。`)
    })
  }

  const handleRemove = (member: DisplayRepoMember) => {
    if (!repoData) return

    void runMemberAction(`remove:${member.userId}`, async () => {
      await removeLibraryMemberAction({
        repoId: repoData.id,
        userId: member.userId,
      })
      setNotice(`${member.displayName} から repository policy を削除しました。`)
    })
  }

  return (
    <div className='container py-6 space-y-6'>
      <div>
        <h1 className='text-2xl font-bold'>設定</h1>
        <p className='text-sm text-muted-foreground'>
          {org}/{repo} の基本情報とメンバー権限を管理します。
        </p>
      </div>

      {error ? (
        <Alert variant='destructive'>
          <ShieldAlert className='h-4 w-4' />
          <AlertTitle>操作に失敗しました</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}

      {notice ? (
        <Alert>
          <AlertTitle>更新しました</AlertTitle>
          <AlertDescription>{notice}</AlertDescription>
        </Alert>
      ) : null}

      {loading ? (
        <Card>
          <CardContent className='py-10 text-center text-sm text-muted-foreground'>
            Loading settings...
          </CardContent>
        </Card>
      ) : !repoData ? (
        <Card>
          <CardContent className='py-10 text-center'>
            <h2 className='text-xl font-semibold'>Repository not found</h2>
          </CardContent>
        </Card>
      ) : (
        <>
          <div className='grid gap-4 md:grid-cols-3'>
            <Card>
              <CardHeader className='pb-3'>
                <CardTitle className='flex items-center gap-2 text-base'>
                  <GitBranch className='h-4 w-4' />
                  Repository
                </CardTitle>
              </CardHeader>
              <CardContent className='space-y-3'>
                <div>
                  <div className='text-xs text-muted-foreground'>Name</div>
                  <div className='font-medium'>{repoData.name}</div>
                </div>
                <div>
                  <div className='text-xs text-muted-foreground'>Description</div>
                  <div className='text-sm'>{repoData.description || '-'}</div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className='pb-3'>
                <CardTitle className='flex items-center gap-2 text-base'>
                  {repoData.isPublic ? <Eye className='h-4 w-4' /> : <Lock className='h-4 w-4' />}
                  Visibility
                </CardTitle>
              </CardHeader>
              <CardContent>
                <Badge variant={repoData.isPublic ? 'secondary' : 'outline'}>
                  {repoData.isPublic ? 'Public' : 'Private'}
                </Badge>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className='pb-3'>
                <CardTitle className='flex items-center gap-2 text-base'>
                  <Users className='h-4 w-4' />
                  Members
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className='text-2xl font-semibold'>{members.length}</div>
                <div className='text-xs text-muted-foreground'>active access grants</div>
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader className='space-y-4'>
              <div className='flex flex-col gap-3 md:flex-row md:items-center md:justify-between'>
                <div>
                  <CardTitle className='text-lg'>メンバー管理</CardTitle>
                  <p className='text-sm text-muted-foreground'>
                    library:reader / library:member / library:admin を付与・変更・削除します。
                  </p>
                </div>
                <Input
                  className='md:w-64'
                  placeholder='メンバーを検索'
                  value={memberQuery}
                  onChange={(event) => setMemberQuery(event.target.value)}
                />
              </div>

              <form className='grid gap-3 md:grid-cols-[1fr_220px_auto]' onSubmit={handleInvite}>
                <Input
                  placeholder='メールアドレスまたはユーザー名'
                  value={inviteTarget}
                  onChange={(event) => setInviteTarget(event.target.value)}
                  disabled={pendingAction === 'invite'}
                />
                <Select
                  value={invitePolicy}
                  onValueChange={(value) => setInvitePolicy(value as LibraryPolicy)}
                  disabled={pendingAction === 'invite'}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {libraryPolicies.map((policy) => (
                      <SelectItem key={policy.id} value={policy.id}>
                        {policy.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Button type='submit' disabled={!inviteTarget.trim() || pendingAction === 'invite'}>
                  <UserPlus className='h-4 w-4' />
                  追加
                </Button>
              </form>
            </CardHeader>
            <CardContent>
              {filteredMembers.length === 0 ? (
                <div className='rounded-md border border-dashed py-10 text-center text-sm text-muted-foreground'>
                  メンバーが見つかりません。
                </div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>ユーザー</TableHead>
                      <TableHead>Policy</TableHead>
                      <TableHead>Source</TableHead>
                      <TableHead className='w-[96px] text-right'>操作</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredMembers.map((member) => (
                      <TableRow key={`${member.permissionSource}:${member.userId}:${member.policyId}`}>
                        <TableCell>
                          <div className='flex min-w-0 items-center gap-3'>
                            <Avatar className='h-9 w-9'>
                              <AvatarImage src={member.image ?? ''} alt={member.displayName} />
                              <AvatarFallback>
                                {member.displayName.charAt(0).toUpperCase()}
                              </AvatarFallback>
                            </Avatar>
                            <div className='min-w-0'>
                              <div className='truncate font-medium'>{member.displayName}</div>
                              <div className='truncate text-xs text-muted-foreground'>
                                {member.email ?? member.userId}
                              </div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell>
                          {member.canManageRepoPolicy ? (
                            <Select
                              value={member.policy}
                              onValueChange={(value) =>
                                handlePolicyChange(member, value as LibraryPolicy)
                              }
                              disabled={pendingAction === `change:${member.userId}`}
                            >
                              <SelectTrigger className='w-[190px]'>
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                {libraryPolicies.map((policy) => (
                                  <SelectItem key={policy.id} value={policy.id}>
                                    {policy.label}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          ) : (
                            <Badge variant='secondary'>{member.policyLabel}</Badge>
                          )}
                        </TableCell>
                        <TableCell>
                          <Badge variant='outline'>{member.permissionSource}</Badge>
                        </TableCell>
                        <TableCell className='text-right'>
                          <Button
                            type='button'
                            variant='ghost'
                            size='icon'
                            aria-label={`${member.displayName} の policy を削除`}
                            disabled={
                              !member.canManageRepoPolicy ||
                              pendingAction === `remove:${member.userId}`
                            }
                            onClick={() => handleRemove(member)}
                          >
                            <Trash2 className='h-4 w-4' />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  )
}
