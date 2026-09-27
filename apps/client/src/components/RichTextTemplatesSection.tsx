import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
} from '@tachyon-sdk/native-ui'
import { FileText, Plus, Trash2 } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import {
  createRichTextTemplate,
  deleteRichTextTemplate,
  fetchRichTextTemplates,
  isRepositoryPermissionError,
  updateRichTextTemplate,
  type RepositorySettingsTarget,
  type RichTextTemplate,
} from '../lib/repositorySettingsApi'
import { useI18n } from '../i18n'
import { RecordBodyEditor } from './RecordBodyEditor'

export function RichTextTemplatesSection({
  target,
  readOnly = false,
  onPermissionDenied,
}: {
  target: RepositorySettingsTarget
  readOnly?: boolean
  onPermissionDenied?: () => void
}) {
  const { t } = useI18n()
  const [templates, setTemplates] = useState<RichTextTemplate[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [newDraftGeneration, setNewDraftGeneration] = useState(0)
  const [name, setName] = useState('')
  const [richText, setRichText] = useState('[]')
  const [editing, setEditing] = useState(false)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [deleteOpen, setDeleteOpen] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      setTemplates(await fetchRichTextTemplates(target))
    } catch (cause) {
      if (isRepositoryPermissionError(cause)) onPermissionDenied?.()
      setError(cause instanceof Error ? cause.message : t('richTextTemplates.loadFailed'))
    } finally {
      setLoading(false)
    }
  }, [onPermissionDenied, t, target])

  useEffect(() => {
    void load()
  }, [load])

  const startNew = () => {
    setNewDraftGeneration((generation) => generation + 1)
    setSelectedId(null)
    setName('')
    setRichText('[]')
    setEditing(true)
    setError(null)
    setNotice(null)
  }

  const selectTemplate = (template: RichTextTemplate) => {
    setSelectedId(template.id)
    setName(template.name)
    setRichText(template.richText)
    setEditing(true)
    setError(null)
    setNotice(null)
  }

  const save = async () => {
    if (readOnly) return
    setBusy(true)
    setError(null)
    setNotice(null)
    try {
      const draft = { name, richText }
      const saved = selectedId
        ? await updateRichTextTemplate(target, selectedId, draft)
        : await createRichTextTemplate(target, draft)
      setTemplates((current) => {
        const next = current.filter((template) => template.id !== saved.id)
        return [...next, saved].sort((left, right) => left.name.localeCompare(right.name))
      })
      setSelectedId(saved.id)
      setName(saved.name)
      setRichText(saved.richText)
      setNotice(t('richTextTemplates.saveSuccess'))
    } catch (cause) {
      if (isRepositoryPermissionError(cause)) onPermissionDenied?.()
      setError(cause instanceof Error ? cause.message : t('richTextTemplates.saveFailed'))
    } finally {
      setBusy(false)
    }
  }

  const remove = async () => {
    if (!selectedId || readOnly) return
    setBusy(true)
    setError(null)
    try {
      await deleteRichTextTemplate(target, selectedId)
      setTemplates((current) => current.filter((template) => template.id !== selectedId))
      setSelectedId(null)
      setName('')
      setRichText('[]')
      setEditing(false)
      setDeleteOpen(false)
      setNotice(null)
    } catch (cause) {
      if (isRepositoryPermissionError(cause)) onPermissionDenied?.()
      setError(cause instanceof Error ? cause.message : t('richTextTemplates.saveFailed'))
    } finally {
      setBusy(false)
    }
  }

  const acknowledgeDraft = useCallback(async () => true, [])

  return (
    <section
      className="mt-5 overflow-hidden rounded-lg border border-border bg-background shadow-soft"
      aria-labelledby="rich-text-templates-heading"
    >
      <div className="flex flex-col gap-3 border-b border-border bg-surface px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-2">
          <FileText className="size-4 text-muted-foreground" aria-hidden="true" />
          <div>
            <h2 id="rich-text-templates-heading" className="text-sm font-semibold">
              {t('richTextTemplates.title')}
            </h2>
            <p className="text-2xs text-muted-foreground">
              {t('richTextTemplates.description')}
            </p>
          </div>
        </div>
        <Button type="button" size="sm" variant="secondary" onClick={startNew} disabled={busy || readOnly}>
          <Plus aria-hidden="true" />
          {t('richTextTemplates.new')}
        </Button>
      </div>

      <div className="grid gap-4 p-4 lg:grid-cols-[220px_minmax(0,1fr)]">
        <div className="space-y-1" aria-label={t('richTextTemplates.title')}>
          {loading ? (
            <p className="px-2 py-3 text-xs text-muted-foreground">{t('common.loading')}</p>
          ) : templates.length === 0 ? (
            <p className="px-2 py-3 text-xs text-muted-foreground">
              {t('richTextTemplates.empty')}
            </p>
          ) : templates.map((template) => (
            <button
              key={template.id}
              type="button"
              aria-current={selectedId === template.id ? 'true' : undefined}
              onClick={() => selectTemplate(template)}
              disabled={busy || readOnly}
              className={`block w-full truncate rounded-md px-2 py-2 text-left text-sm ${selectedId === template.id ? 'bg-selected text-primary' : 'text-foreground hover:bg-muted'}`}
            >
              {template.name}
            </button>
          ))}
        </div>

        {editing ? (
          <div className="min-w-0 space-y-3">
            <div className="space-y-1.5">
              <Label htmlFor="rich-text-template-name">{t('richTextTemplates.name')}</Label>
              <Input
                id="rich-text-template-name"
                value={name}
                onChange={(event) => setName(event.target.value)}
                maxLength={255}
                disabled={busy || readOnly}
              />
            </div>
            <div className="space-y-1.5">
              <Label>{t('richTextTemplates.body')}</Label>
              <RecordBodyEditor
                key={selectedId ?? `new-rich-text-template-${newDraftGeneration}`}
                value={richText}
                format="richText"
                editable={!busy && !readOnly}
                surface="panel"
                imageTarget={{
                  org: target.orgUsername,
                  repo: target.repoUsername,
                  operatorId: target.operatorId,
                }}
                onDraftChange={setRichText}
                onCommit={acknowledgeDraft}
              />
            </div>
            {error ? (
              <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
                {error}
              </p>
            ) : null}
            {notice ? (
              <p role="status" className="text-xs text-success">{notice}</p>
            ) : null}
            <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border pt-3">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={!selectedId || busy || readOnly}
                onClick={() => setDeleteOpen(true)}
              >
                <Trash2 aria-hidden="true" />
                {t('common.delete')}
				</Button>
				<div className="flex gap-2">
					<Button
						type="button"
						variant="ghost"
						size="sm"
						disabled={busy}
						onClick={() => setEditing(false)}
					>
						{t('common.cancel')}
					</Button>
                <Button type="button" size="sm" disabled={busy || readOnly || !name.trim()} onClick={() => void save()}>
                  {t('common.save')}
                </Button>
              </div>
            </div>
          </div>
        ) : (
          <p className="flex min-h-32 items-center justify-center rounded-md border border-dashed border-border px-4 text-center text-sm text-muted-foreground">
            {t('richTextTemplates.empty')}
          </p>
        )}
      </div>

      <Dialog open={deleteOpen} onOpenChange={(open) => !busy && setDeleteOpen(open)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{t('richTextTemplates.deleteTitle')}</DialogTitle>
            <DialogDescription>{t('richTextTemplates.deleteDescription')}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="ghost" disabled={busy}>{t('common.cancel')}</Button>
            </DialogClose>
            <Button type="button" variant="destructive" disabled={busy} onClick={() => void remove()}>
              <Trash2 aria-hidden="true" />
              {t('common.delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  )
}
