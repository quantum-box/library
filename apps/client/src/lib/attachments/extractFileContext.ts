import JSZip, { type JSZipObject } from 'jszip'
import * as XLSX from 'xlsx'
import { detectFileType, type FileType } from '../../components/files/types'

const DEFAULT_MAX_CONTEXT_CHARS = 16_000
const MAX_ATTACHMENT_BYTES = 50 * 1024 * 1024
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

async function readXmlEntry(entry: JSZipObject, type: FileType) {
  const xml = await entry.async('string')
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
  requireZipEntry(zip, '[Content_Types].xml', 'pptx')

  const presentationPath = 'ppt/presentation.xml'
  const presentationXml = await readXmlEntry(
    requireZipEntry(zip, presentationPath, 'pptx'),
    'pptx',
  )
  const relationshipsXml = await readXmlEntry(
    requireZipEntry(zip, 'ppt/_rels/presentation.xml.rels', 'pptx'),
    'pptx',
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

    const slideXml = await readXmlEntry(requireZipEntry(zip, slidePath, 'pptx'), 'pptx')
    const body = extractSlideBodyText(slideXml)
    if (body) slides.push(`## Slide ${index + 1}\n${body}`)
  }

  return slides.join('\n\n')
}

async function extractPdfText(file: File) {
  const bytes = new Uint8Array(await file.arrayBuffer())
  const header = new TextDecoder('ascii').decode(bytes.slice(0, Math.min(bytes.length, 1024)))
  if (!header.includes('%PDF-')) throw unreadableAttachment('pdf')

  // The legacy build supplies Node-safe fallbacks while remaining browser-compatible.
  const pdfjsLib = await import('pdfjs-dist/legacy/build/pdf.mjs')
  const loadingTask = pdfjsLib.getDocument({
    data: bytes,
    isEvalSupported: false,
    useSystemFonts: true,
  })
  try {
    const pdf = await loadingTask.promise
    const pages: string[] = []
    for (let pageNumber = 1; pageNumber <= pdf.numPages; pageNumber += 1) {
      const page = await pdf.getPage(pageNumber)
      const content = await page.getTextContent()
      let pageText = ''
      for (const item of content.items) {
        if (!('str' in item)) continue
        pageText += item.str
        pageText += item.hasEOL ? '\n' : ' '
      }
      const normalizedPage = pageText.trim()
      if (normalizedPage) pages.push(`## Page ${pageNumber}\n${normalizedPage}`)
      page.cleanup()
    }
    return pages.join('\n\n')
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
  }

  const workbook = XLSX.read(buffer, { type: 'array', cellDates: true })
  if (workbook.SheetNames.length === 0) throw unreadableAttachment('excel')
  return workbook.SheetNames.map((sheetName) => {
    const sheet = workbook.Sheets[sheetName]
    if (!sheet) throw unreadableAttachment('excel')
    return `# ${sheetName}\n${XLSX.utils.sheet_to_csv(sheet)}`.trimEnd()
  }).join('\n\n')
}

async function extractDocxText(file: File) {
  const buffer = await file.arrayBuffer()
  const zip = await loadOfficeZip(buffer, 'docx')
  requireZipEntry(zip, '[Content_Types].xml', 'docx')
  const documentXml = await readXmlEntry(
    requireZipEntry(zip, 'word/document.xml', 'docx'),
    'docx',
  )
  return extractDocxBodyText(documentXml)
}

export async function extractFileContext(
  file: File,
  maxChars = DEFAULT_MAX_CONTEXT_CHARS,
) {
  const type = detectFileType(file)
  if (type === 'unknown') return ''

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
        content = await extractPdfText(file)
        break
      case 'pptx':
        content = await extractPptxText(file)
        break
    }

    return compactText(content, Math.max(0, Math.floor(maxChars)))
  } catch (error) {
    if (error instanceof FileContextExtractionError) throw error
    throw unreadableAttachment(type)
  }
}
