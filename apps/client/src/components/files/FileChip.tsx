import { FileSpreadsheet, FileText, Paperclip, Presentation, X } from 'lucide-react'
import { type FileAttachment, detectAttachmentFileType, formatFileSize, getFileColor } from './types'
import { useI18n } from '../../i18n'

interface FileChipProps {
  file: FileAttachment
  onPreview: (file: FileAttachment) => void
  onRemove?: (fileId: string) => void
}

export function FileChip({ file, onPreview, onRemove }: FileChipProps) {
  const { t } = useI18n()
  const fileType = detectAttachmentFileType(file)
  const color = getFileColor(fileType)
  const ext = file.name.split('.').pop()?.toUpperCase() ?? ''
  const FileIcon = fileType === 'excel' || fileType === 'csv'
    ? FileSpreadsheet
    : fileType === 'pptx'
      ? Presentation
      : fileType === 'unknown'
        ? Paperclip
        : FileText

  return (
    <div
      className="group/chip inline-flex max-w-full items-center gap-1 rounded-lg border border-border bg-surface p-1 transition-colors hover:bg-surface-hover sm:gap-2"
    >
      <button
        type="button"
        aria-label={t('files.previewNamed', { name: file.name })}
        className="flex min-w-0 items-center gap-2 rounded px-1 py-1 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 sm:px-2"
        onClick={() => onPreview(file)}
      >
        <span
          aria-hidden="true"
          className="w-8 h-8 rounded flex items-center justify-center text-sm flex-shrink-0"
          style={{ background: `${color}20`, color }}
        >
          <FileIcon className="size-4" aria-hidden="true" />
        </span>
        <span className="min-w-0">
          <span className="block max-w-[11rem] truncate text-xs font-medium text-foreground">
            {file.name}
          </span>
          <span className="block text-xs text-subtle">
            {ext} · {formatFileSize(file.size)}
          </span>
        </span>
      </button>
      {onRemove && (
        <button
          type="button"
          aria-label={t('files.removeNamed', { name: file.name })}
          onClick={(e) => { e.stopPropagation(); onRemove(file.id) }}
          className="w-7 h-7 shrink-0 rounded flex items-center justify-center text-xs opacity-100 transition-opacity cursor-pointer text-subtle hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 sm:opacity-0 sm:group-hover/chip:opacity-100 sm:group-focus-within/chip:opacity-100"
        >
          <X className="size-3.5" aria-hidden="true" />
        </button>
      )}
    </div>
  )
}
