import { useEffect, useState } from 'react'
import type { Meta, StoryObj } from '@storybook/react-vite'
import { Badge, Button, TooltipProvider } from '@tachyon-sdk/native-ui'
import { ArrowLeft, Check, Database, Link2, Trash2 } from 'lucide-react'
import { I18nProvider, loadCatalog, useI18n } from '../i18n'
import { DataEditorOfflineIndicator } from './DataEditorOfflineIndicator'

/**
 * The data screen app bar, with the Live offline indicator in the same place
 * DataEditorPage renders it. Japanese, so the short label and the tooltip
 * match the copy the data screen shows.
 */
function OfflineAppBar() {
  const [ready, setReady] = useState(false)

  useEffect(() => {
    void loadCatalog('ja').then(() => setReady(true))
  }, [])

  if (!ready) return <div className="h-12 w-[920px] bg-background" />

  return (
    <I18nProvider initial="ja">
      <TooltipProvider delayDuration={0}>
        <OfflineAppBarChrome />
      </TooltipProvider>
    </I18nProvider>
  )
}

function OfflineAppBarChrome() {
  const { t } = useI18n()

  return (
    <header className="flex h-12 w-[920px] items-center gap-2 border-b border-border bg-background px-4 text-foreground">
      <Button
        variant="ghost"
        size="icon"
        className="size-7"
        aria-label={t('detail.backToData')}
      >
        <ArrowLeft className="size-4" aria-hidden="true" />
      </Button>
      <Database className="size-4 shrink-0 text-primary" aria-hidden="true" />
      <div className="flex min-w-0 items-center gap-1 text-sm">
        <span className="max-w-28 truncate text-muted-foreground">quantum-box</span>
        <span className="text-subtle-foreground">/</span>
        <span className="max-w-36 truncate text-muted-foreground">photon-core</span>
        <span className="text-subtle-foreground">/</span>
        <span className="truncate font-mono text-xs font-medium">seed-data-201</span>
      </div>
      <Badge variant="outline">{t('repository.tab.data')}</Badge>
      <div className="ml-auto flex shrink-0 items-center gap-1.5">
        <DataEditorOfflineIndicator offline />
        <span className="flex items-center gap-1 text-xs text-muted-foreground">
          <Check className="size-3.5 text-status-done" aria-hidden="true" />
          {t('common.saved')}
        </span>
        <Button variant="ghost" size="icon" className="size-7 text-muted-foreground" aria-label={t('share.shareData')}>
          <Link2 className="size-3.5" aria-hidden="true" />
        </Button>
        <Button variant="ghost" size="icon" className="size-7 text-muted-foreground" aria-label={t('dataEditor.deleteData')}>
          <Trash2 className="size-3.5" aria-hidden="true" />
        </Button>
      </div>
    </header>
  )
}

const meta = {
  title: 'Library/DataEditorOfflineIndicator',
  component: OfflineAppBar,
} satisfies Meta<typeof OfflineAppBar>

export default meta
type Story = StoryObj<typeof meta>

export const InAppBar: Story = {}
