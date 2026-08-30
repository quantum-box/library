import { formatNumber, getActiveLocale } from '../../i18n'

export interface FileAttachment {
  id: string
  name: string
  size: number
  type: string
  url?: string // object URL or remote download URL
  file?: File
  previewType?: FileType
}

export type FileType = 'pdf' | 'excel' | 'csv' | 'docx' | 'pptx' | 'unknown'

export function detectFileType(file: File): FileType {
  const ext = file.name.split('.').pop()?.toLowerCase() ?? ''
  const mime = file.type

  if (ext === 'pdf' || mime === 'application/pdf') return 'pdf'
  if (ext === 'xlsx' || ext === 'xls' || mime.includes('spreadsheet') || mime.includes('excel'))
    return 'excel'
  if (ext === 'csv' || mime === 'text/csv') return 'csv'
  if (ext === 'docx' || mime.includes('wordprocessingml')) return 'docx'
  if (ext === 'pptx' || mime.includes('presentationml')) return 'pptx'
  return 'unknown'
}

export function detectAttachmentFileType(file: Pick<FileAttachment, 'name' | 'type' | 'previewType'>): FileType {
  if (file.previewType) return file.previewType
  return detectFileType(new File([], file.name, { type: file.type }))
}

/**
 * Size label for a chip or list row. Unit suffixes stay SI, which reads the
 * same everywhere; only the digits are formatted for the active locale, so
 * the decimal separator follows it.
 */
export function formatFileSize(bytes: number): string {
  const locale = getActiveLocale()
  const oneDecimal = { minimumFractionDigits: 1, maximumFractionDigits: 1 } as const
  if (bytes < 1024) return `${formatNumber(locale, bytes)} B`
  if (bytes < 1024 * 1024) return `${formatNumber(locale, bytes / 1024, oneDecimal)} KB`
  return `${formatNumber(locale, bytes / (1024 * 1024), oneDecimal)} MB`
}

export type FileIconName = 'document' | 'spreadsheet' | 'presentation' | 'attachment'

const FILE_ICONS: Record<FileType, FileIconName> = {
  pdf: 'document',
  excel: 'spreadsheet',
  csv: 'spreadsheet',
  docx: 'document',
  pptx: 'presentation',
  unknown: 'attachment',
}

const FILE_COLORS: Record<FileType, string> = {
  pdf: '#e74c3c',
  excel: '#27ae60',
  csv: '#27ae60',
  docx: '#2980b9',
  pptx: '#e67e22',
  unknown: '#8a8a9a',
}

export function getFileIcon(type: FileType): FileIconName {
  return FILE_ICONS[type]
}

export function getFileColor(type: FileType): string {
  return FILE_COLORS[type]
}
