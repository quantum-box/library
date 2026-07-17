import JSZip, { type JSZipObject } from 'jszip'
import * as XLSX from 'xlsx'
import { detectFileType, type FileType } from '../../components/files/types'
import { loadPdfJs } from '../pdfJsWorker'

const DEFAULT_MAX_CONTEXT_CHARS = 16_000
const MAX_ATTACHMENT_BYTES = 50 * 1024 * 1024
const MAX_OFFICE_XML_BYTES = 5_000_000
const MAX_WORKBOOK_EXPANDED_BYTES = 20 * 1024 * 1024
const MAX_WORKBOOK_SHEETS = 50
const MAX_WORKBOOK_ROWS = 1_000
const MAX_WORKBOOK_COLUMNS = 256
const MAX_WORKBOOK_CELLS_PER_SHEET = 250_000
const MAX_WORKBOOK_CELLS_TOTAL = 500_000
const MAX_PDF_PAGES = 200
const MAX_PDF_EXTRACTED_CHARS = 1_000_000
const MAX_XML_ENTRY_CHARS = 5_000_000
const DRAWINGML_NAMESPACE = 'http://schemas.openxmlformats.org/drawingml/2006/main'
const WORDPROCESSINGML_NAMESPACE = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'

export class FileContextExtractionError extends Error {
  readonly code: 'file-too-large' | 'unreadable-file'

  constructor(code: 'file-too-large' | 'unreadable-file', message: string) {
    super(message)
    this.name = 'FileContextExtractionError'
    this.code = code
  }
}

function compactText(value: string, maxChars: number) {
  const normalized = value.split('\u0000').join('').replace(/\r\n?/g, '\n').trim()
  if (normalized.length <= maxChars) return normalized
  return `${normalized.slice(0, maxChars)}\n\n[Content truncated]`
}

function unreadableAttachment(type: FileType): FileContextExtractionError {
  return new FileContextExtractionError(
    'unreadable-file',
    `Unable to extract text from this ${type.toUpperCase()} attachment. The file may be encrypted or damaged.`,
  )
}

function assertExtractableSize(file: File) {
  if (file.size === 0) throw unreadableAttachment(detectFileType(file))
  if (file.size > MAX_ATTACHMENT_BYTES) {
    throw new FileContextExtractionError(
      'file-too-large',
      'This attachment is too large to extract safely.',
    )
  }
}

function hasZipSignature(bytes: Uint8Array) {
  return bytes.length >= 4 &&
    bytes[0] === 0x50 &&
    bytes[1] === 0x4b &&
    ((bytes[2] === 0x03 && bytes[3] === 0x04) ||
      (bytes[2] === 0x05 && bytes[3] === 0x06) ||
      (bytes[2] === 0x07 && bytes[3] === 0x08))
}

function hasCompoundFileSignature(bytes: Uint8Array) {
  const signature = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]
  return signature.every((value, index) => bytes[index] === value)
}

async function loadOfficeZip(buffer: ArrayBuffer, type: FileType) {
  if (!hasZipSignature(new Uint8Array(buffer))) throw unreadableAttachment(type)
  return JSZip.loadAsync(buffer)
}

function requireZipEntry(zip: JSZip, path: string, type: FileType) {
  const entry = zip.file(path)
  if (!entry || entry.dir) throw unreadableAttachment(type)
  return entry
}

interface ZipEntryStream {
  on(event: 'data', listener: (chunk: Uint8Array) => void): this
  on(event: 'error', listener: (error: unknown) => void): this
  on(event: 'end', listener: () => void): this
  pause(): this
  resume(): this
}

interface ZipExpansionBudget {
  remainingBytes: number
}

function zipEntryUncompressedSize(entry: JSZipObject) {
  const uncompressedSize = (entry as JSZipObject & {
    _data?: { uncompressedSize?: unknown }
  })._data?.uncompressedSize
  if (uncompressedSize === undefined) return undefined
  if (
    typeof uncompressedSize !== 'number' ||
    !Number.isSafeInteger(uncompressedSize) ||
    uncompressedSize < 0
  ) {
    return null
  }
  return uncompressedSize
}

function assertDeclaredSizeFitsBudget(
  entry: JSZipObject,
  budget: ZipExpansionBudget,
  type: FileType,
) {
  const uncompressedSize = zipEntryUncompressedSize(entry)
  if (
    uncompressedSize === null ||
    (uncompressedSize !== undefined &&
      uncompressedSize > budget.remainingBytes)
  ) {
    throw unreadableAttachment(type)
  }
}

function consumeZipEntryWithBudget(
  entry: JSZipObject,
  type: FileType,
  budget: ZipExpansionBudget,
  collect: false,
): Promise<void>
function consumeZipEntryWithBudget(
  entry: JSZipObject,
  type: FileType,
  budget: ZipExpansionBudget,
  collect: true,
): Promise<Uint8Array>
function consumeZipEntryWithBudget(
  entry: JSZipObject,
  type: FileType,
  budget: ZipExpansionBudget,
  collect: boolean,
) {
  assertDeclaredSizeFitsBudget(entry, budget, type)

  return new Promise<Uint8Array | void>((resolve, reject) => {
    let settled = false
    let totalBytes = 0
    const chunks: Uint8Array[] = []
    const stream = (entry as JSZipObject & {
      internalStream(outputType: 'uint8array'): ZipEntryStream
    }).internalStream('uint8array')

    const rejectOnce = (error: unknown) => {
      if (settled) return
      settled = true
      chunks.length = 0
      reject(error)
    }
    const abort = (error: Error) => {
      rejectOnce(error)
      stream.pause()
    }

    stream
      .on('data', (chunk) => {
        if (settled) return
        if (!(chunk instanceof Uint8Array)) {
          abort(unreadableAttachment(type))
          return
        }
        if (chunk.byteLength > budget.remainingBytes) {
          abort(unreadableAttachment(type))
          return
        }
        budget.remainingBytes -= chunk.byteLength
        totalBytes += chunk.byteLength
        if (collect) chunks.push(chunk)
      })
      .on('error', rejectOnce)
      .on('end', () => {
        if (settled) return
        settled = true
        if (!collect) {
          resolve()
          return
        }
        const bytes = new Uint8Array(totalBytes)
        let offset = 0
        for (const chunk of chunks) {
          bytes.set(chunk, offset)
          offset += chunk.byteLength
        }
        chunks.length = 0
        resolve(bytes)
      })
      .resume()
  })
}

async function readXmlEntry(
  entry: JSZipObject,
  type: FileType,
  budget: ZipExpansionBudget,
) {
  let xml: string
  try {
    xml = new TextDecoder('utf-8', { fatal: true }).decode(
      await consumeZipEntryWithBudget(entry, type, budget, true),
    )
  } catch (error) {
    if (error instanceof FileContextExtractionError) throw error
    throw unreadableAttachment(type)
  }
  if (
    xml.length > MAX_XML_ENTRY_CHARS ||
    /<!DOCTYPE|<!ENTITY/i.test(xml)
  ) {
    throw unreadableAttachment(type)
  }
  return xml
}

function parseXml(xml: string, type: FileType) {
  const document = new DOMParser().parseFromString(xml, 'application/xml')
  const hasParserError = Array.from(document.getElementsByTagName('*'))
    .some((element) => element.localName === 'parsererror')
  if (hasParserError) throw unreadableAttachment(type)
  return document
}

function elementsByLocalName(document: XMLDocument | Element, localName: string) {
  return Array.from(document.getElementsByTagName('*'))
    .filter((element) => element.localName === localName)
}

function resolvePackagePath(sourcePath: string, targetPath: string) {
  const target = decodeURI(targetPath).replace(/\\/g, '/')
  const parts = target.startsWith('/')
    ? []
    : sourcePath.split('/').slice(0, -1)

  for (const part of target.replace(/^\//, '').split('/')) {
    if (!part || part === '.') continue
    if (part === '..') {
      if (parts.length === 0) throw unreadableAttachment('pptx')
      parts.pop()
    } else {
      parts.push(part)
    }
  }

  return parts.join('/')
}

function textFromDrawingParagraph(paragraph: Element) {
  let result = ''

  const visit = (element: Element) => {
    for (const child of Array.from(element.children)) {
      if (child.namespaceURI === DRAWINGML_NAMESPACE && child.localName === 't') {
        result += child.textContent ?? ''
      } else if (child.namespaceURI === DRAWINGML_NAMESPACE && child.localName === 'br') {
        result += '\n'
      } else if (child.namespaceURI === DRAWINGML_NAMESPACE && child.localName === 'tab') {
        result += '\t'
      } else {
        visit(child)
      }
    }
  }

  visit(paragraph)
  return result.trim()
}

function extractSlideBodyText(xml: string) {
  const document = parseXml(xml, 'pptx')
  return elementsByLocalName(document, 'p')
    .filter((element) => element.namespaceURI === DRAWINGML_NAMESPACE)
    .map(textFromDrawingParagraph)
    .filter(Boolean)
    .join('\n')
}

function textFromWordParagraph(paragraph: Element) {
  let result = ''

  const visit = (element: Element) => {
    for (const child of Array.from(element.children)) {
      if (child.namespaceURI === WORDPROCESSINGML_NAMESPACE && child.localName === 't') {
        result += child.textContent ?? ''
      } else if (child.namespaceURI === WORDPROCESSINGML_NAMESPACE && child.localName === 'tab') {
        result += '\t'
      } else if (
        child.namespaceURI === WORDPROCESSINGML_NAMESPACE &&
        (child.localName === 'br' || child.localName === 'cr')
      ) {
        result += '\n'
      } else {
        visit(child)
      }
    }
  }

  visit(paragraph)
  return result.trim()
}

function extractDocxBodyText(xml: string) {
  const document = parseXml(xml, 'docx')
  return elementsByLocalName(document, 'p')
    .filter((element) => element.namespaceURI === WORDPROCESSINGML_NAMESPACE)
    .map(textFromWordParagraph)
    .filter(Boolean)
    .join('\n')
}

async function extractPptxText(file: File) {
  const buffer = await file.arrayBuffer()
  const zip = await loadOfficeZip(buffer, 'pptx')
  const expansionBudget = { remainingBytes: MAX_OFFICE_XML_BYTES }
  requireZipEntry(zip, '[Content_Types].xml', 'pptx')

  const presentationPath = 'ppt/presentation.xml'
  const presentationXml = await readXmlEntry(
    requireZipEntry(zip, presentationPath, 'pptx'),
    'pptx',
    expansionBudget,
  )
  const relationshipsXml = await readXmlEntry(
    requireZipEntry(zip, 'ppt/_rels/presentation.xml.rels', 'pptx'),
    'pptx',
    expansionBudget,
  )
  const presentation = parseXml(presentationXml, 'pptx')
  const relationships = parseXml(relationshipsXml, 'pptx')

  const slideTargets = new Map<string, string>()
  for (const relationship of elementsByLocalName(relationships, 'Relationship')) {
    if (relationship.getAttribute('TargetMode') === 'External') continue
    const id = relationship.getAttribute('Id')
    const target = relationship.getAttribute('Target')
    const relationshipType = relationship.getAttribute('Type')
    if (id && target && relationshipType?.endsWith('/slide')) {
      slideTargets.set(id, resolvePackagePath(presentationPath, target))
    }
  }

  const slides: string[] = []
  const slideIds = elementsByLocalName(presentation, 'sldId')
  for (const [index, slideId] of slideIds.entries()) {
    const relationshipId = slideId.getAttribute('r:id') ??
      Array.from(slideId.attributes).find((attribute) =>
        attribute.localName === 'id' && attribute.namespaceURI?.endsWith('/relationships')
      )?.value
    const slidePath = relationshipId ? slideTargets.get(relationshipId) : undefined
    if (!slidePath) throw unreadableAttachment('pptx')

    const slideXml = await readXmlEntry(
      requireZipEntry(zip, slidePath, 'pptx'),
      'pptx',
      expansionBudget,
    )
    const body = extractSlideBodyText(slideXml)
    if (body) slides.push(`## Slide ${index + 1}\n${body}`)
  }

  return slides.join('\n\n')
}

async function extractPdfText(file: File, maxChars: number) {
  const bytes = new Uint8Array(await file.arrayBuffer())
  const header = new TextDecoder('ascii').decode(bytes.slice(0, Math.min(bytes.length, 1024)))
  if (!header.includes('%PDF-')) throw unreadableAttachment('pdf')

  const pdfjsLib = await loadPdfJs()
  const loadingTask = pdfjsLib.getDocument({
    data: bytes,
    isEvalSupported: false,
    useSystemFonts: true,
  })
  try {
    const pdf = await loadingTask.promise
    const pages: string[] = []
    const extractionLimit = Math.min(
      MAX_PDF_EXTRACTED_CHARS,
      Math.max(1, maxChars + 1),
    )
    let extractedChars = 0
    let truncated = false
    const pageCount = Math.min(pdf.numPages, MAX_PDF_PAGES)

    for (let pageNumber = 1; pageNumber <= pageCount; pageNumber += 1) {
      const page = await pdf.getPage(pageNumber)
      try {
        const content = await page.getTextContent()
        const pageChunks: string[] = []
        let pageChars = 0
        const separatorChars = pages.length === 0 ? 0 : 2
        const pageHeader = `## Page ${pageNumber}\n`
        const pageBudget = Math.max(
          0,
          extractionLimit - extractedChars - separatorChars - pageHeader.length,
        )

        for (const item of content.items) {
          if (!('str' in item)) continue
          const value = `${item.str}${item.hasEOL ? '\n' : ' '}`
          const remaining = pageBudget - pageChars
          if (remaining <= 0) {
            truncated = true
            break
          }
          pageChunks.push(value.slice(0, remaining))
          pageChars += Math.min(value.length, remaining)
          if (value.length > remaining) {
            truncated = true
            break
          }
        }

        const normalizedPage = pageChunks.join('').trim()
        if (normalizedPage) {
          const renderedPage = `${pageHeader}${normalizedPage}`
          extractedChars += separatorChars + renderedPage.length
          pages.push(renderedPage)
        }
      } finally {
        page.cleanup()
      }

      if (truncated || extractedChars >= extractionLimit) {
        truncated = true
        break
      }
    }

    if (pdf.numPages > pageCount) truncated = true

    const output = pages.join('\n\n')
    return truncated
      ? `${output}${output ? '\n\n' : ''}[Content truncated]`
      : output
  } finally {
    await loadingTask.destroy()
  }
}

async function extractCsvText(file: File) {
  const bytes = new Uint8Array(await file.arrayBuffer())
  const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes).replace(/^\uFEFF/, '')
  if (text.includes('\u0000')) throw unreadableAttachment('csv')
  return text
}

async function extractWorkbookText(file: File) {
  const buffer = await file.arrayBuffer()
  const bytes = new Uint8Array(buffer)
  const extension = file.name.split('.').pop()?.toLowerCase()

  if (extension === 'xls') {
    if (!hasCompoundFileSignature(bytes)) throw unreadableAttachment('excel')
  } else {
    const zip = await loadOfficeZip(buffer, 'excel')
    requireZipEntry(zip, '[Content_Types].xml', 'excel')
    requireZipEntry(zip, 'xl/workbook.xml', 'excel')
    const entries = Object.values(zip.files).filter((entry) => !entry.dir)
    const expansionBudget = { remainingBytes: MAX_WORKBOOK_EXPANDED_BYTES }

    // The central-directory values provide a cheap fail-fast check. Streaming
    // every entry against the same budget then enforces the bound even when a
    // malicious archive understates those values.
    let declaredTotal = 0
    for (const entry of entries) {
      const uncompressedSize = zipEntryUncompressedSize(entry)
      if (
        uncompressedSize === null ||
        (uncompressedSize !== undefined &&
          uncompressedSize > MAX_WORKBOOK_EXPANDED_BYTES - declaredTotal)
      ) {
        throw unreadableAttachment('excel')
      }
      declaredTotal += uncompressedSize ?? 0
    }
    for (const entry of entries) {
      await consumeZipEntryWithBudget(entry, 'excel', expansionBudget, false)
    }
  }

  const workbook = XLSX.read(buffer, {
    type: 'array',
    cellDates: true,
    sheetRows: MAX_WORKBOOK_ROWS,
  })
  if (
    workbook.SheetNames.length === 0 ||
    workbook.SheetNames.length > MAX_WORKBOOK_SHEETS
  ) {
    throw unreadableAttachment('excel')
  }
  let totalCells = 0
  return workbook.SheetNames.map((sheetName) => {
    const sheet = workbook.Sheets[sheetName]
    if (!sheet) throw unreadableAttachment('excel')
    if (sheet['!ref']) {
      let range: ReturnType<typeof XLSX.utils.decode_range>
      try {
        range = XLSX.utils.decode_range(sheet['!ref'])
      } catch {
        throw unreadableAttachment('excel')
      }
      const rows = range.e.r - range.s.r + 1
      const columns = range.e.c - range.s.c + 1
      if (
        ![range.s.r, range.s.c, range.e.r, range.e.c].every(
          (coordinate) => Number.isSafeInteger(coordinate) && coordinate >= 0,
        ) ||
        !Number.isSafeInteger(rows) ||
        !Number.isSafeInteger(columns) ||
        rows <= 0 ||
        columns <= 0 ||
        rows > MAX_WORKBOOK_ROWS ||
        columns > MAX_WORKBOOK_COLUMNS ||
        rows > Math.floor(MAX_WORKBOOK_CELLS_PER_SHEET / columns)
      ) {
        throw unreadableAttachment('excel')
      }
      const cells = rows * columns
      if (cells > MAX_WORKBOOK_CELLS_TOTAL - totalCells) {
        throw unreadableAttachment('excel')
      }
      totalCells += cells
    }
    return `# ${sheetName}\n${XLSX.utils.sheet_to_csv(sheet)}`.trimEnd()
  }).join('\n\n')
}

async function extractDocxText(file: File) {
  const buffer = await file.arrayBuffer()
  const zip = await loadOfficeZip(buffer, 'docx')
  const expansionBudget = { remainingBytes: MAX_OFFICE_XML_BYTES }
  requireZipEntry(zip, '[Content_Types].xml', 'docx')
  const documentXml = await readXmlEntry(
    requireZipEntry(zip, 'word/document.xml', 'docx'),
    'docx',
    expansionBudget,
  )
  return extractDocxBodyText(documentXml)
}

export async function extractFileContext(
  file: File,
  maxChars = DEFAULT_MAX_CONTEXT_CHARS,
) {
  const type = detectFileType(file)
  if (type === 'unknown') return ''
  const contextCharLimit = Number.isFinite(maxChars)
    ? Math.max(0, Math.floor(maxChars))
    : DEFAULT_MAX_CONTEXT_CHARS

  try {
    assertExtractableSize(file)
    let content = ''

    switch (type) {
      case 'csv':
        content = await extractCsvText(file)
        break
      case 'excel':
        content = await extractWorkbookText(file)
        break
      case 'docx':
        content = await extractDocxText(file)
        break
      case 'pdf':
        content = await extractPdfText(file, contextCharLimit)
        break
      case 'pptx':
        content = await extractPptxText(file)
        break
    }

    return compactText(content, contextCharLimit)
  } catch (error) {
    if (error instanceof FileContextExtractionError) throw error
    throw unreadableAttachment(type)
  }
}
