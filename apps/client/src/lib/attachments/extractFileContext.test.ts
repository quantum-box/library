import JSZip from 'jszip'
import * as XLSX from 'xlsx'
import { describe, expect, it, vi } from 'vitest'
import { configurePdfJsWorker, PDF_WORKER_URL } from '../pdfJsWorker'
import { FileContextExtractionError, extractFileContext } from './extractFileContext'

if (!('DOMMatrix' in globalThis)) {
  class TestDOMMatrix {
    a = 1
    b = 0
    c = 0
    d = 1
    e = 0
    f = 0
  }
  Object.defineProperty(globalThis, 'DOMMatrix', {
    configurable: true,
    value: TestDOMMatrix,
  })
}

function createPdfDocument(pageLines: string[][]) {
  const fontId = 3 + pageLines.length * 2
  const pageIds = pageLines.map((_, index) => 3 + index * 2)
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    `<< /Type /Pages /Kids [${pageIds.map((id) => `${id} 0 R`).join(' ')}] /Count ${pageIds.length} >>`,
  ]

  for (const [index, lines] of pageLines.entries()) {
    const pageId = pageIds[index]
    const contentId = pageId + 1
    const textCommands = lines.flatMap((line, lineIndex) => [
      ...(lineIndex === 0 ? [] : ['0 -24 Td']),
      `(${line.replace(/[\\()]/g, '\\$&')}) Tj`,
    ])
    const stream = ['BT', '/F1 18 Tf', '72 720 Td', ...textCommands, 'ET'].join('\n')
    objects.push(
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 ${fontId} 0 R >> >> /Contents ${contentId} 0 R >>`,
      `<< /Length ${stream.length} >>\nstream\n${stream}\nendstream`,
    )
  }
  objects.push('<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>')

  let pdf = '%PDF-1.4\n'
  const offsets = [0]
  for (const [index, object] of objects.entries()) {
    offsets.push(pdf.length)
    pdf += `${index + 1} 0 obj\n${object}\nendobj\n`
  }
  const xrefOffset = pdf.length
  pdf += `xref\n0 ${objects.length + 1}\n`
  pdf += '0000000000 65535 f \n'
  pdf += offsets.slice(1).map((offset) => `${String(offset).padStart(10, '0')} 00000 n \n`).join('')
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\n`
  pdf += `startxref\n${xrefOffset}\n%%EOF\n`
  return pdf
}

function createPdfFixture() {
  return createPdfDocument([['Quarterly report', 'Revenue up']])
}

async function createXlsxWithWorksheetDimension(dimension: string) {
  const workbook = XLSX.utils.book_new()
  XLSX.utils.book_append_sheet(workbook, XLSX.utils.aoa_to_sheet([['Value']]), 'Sheet1')
  const buffer = XLSX.write(workbook, { type: 'array', bookType: 'xlsx' }) as ArrayBuffer
  const zip = await JSZip.loadAsync(buffer)
  const entry = zip.file('xl/worksheets/sheet1.xml')
  if (!entry) throw new Error('Missing worksheet fixture')
  const xml = await entry.async('string')
  const updated = xml.replace(/<dimension ref="[^"]*"\/>/, `<dimension ref="${dimension}"/>`)
  if (updated === xml) throw new Error('Missing worksheet dimension')
  zip.file('xl/worksheets/sheet1.xml', updated)
  return zip.generateAsync({ type: 'arraybuffer' })
}

async function createDocxFixture() {
  const zip = new JSZip()
  zip.file('[Content_Types].xml', `<?xml version="1.0" encoding="UTF-8"?>
    <Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
      <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
      <Default Extension="xml" ContentType="application/xml"/>
      <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
    </Types>`)
  zip.file('_rels/.rels', `<?xml version="1.0" encoding="UTF-8"?>
    <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
      <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
    </Relationships>`)
  zip.file('word/_rels/document.xml.rels', `<?xml version="1.0" encoding="UTF-8"?>
    <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>`)
  zip.file('word/document.xml', `<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
      <w:body>
        <w:p><w:r><w:t>Project brief</w:t></w:r></w:p>
        <w:p><w:r><w:t>Ship the client safely</w:t></w:r></w:p>
        <w:sectPr/>
      </w:body>
    </w:document>`)
  return zip.generateAsync({ type: 'arraybuffer' })
}

function forgeCentralDirectorySize(
  archive: ArrayBuffer,
  entryName: string,
  uncompressedSize: number,
) {
  const bytes = new Uint8Array(archive.slice(0))
  const view = new DataView(bytes.buffer)
  const decoder = new TextDecoder()

  for (let offset = 0; offset <= bytes.byteLength - 46;) {
    if (view.getUint32(offset, true) !== 0x02014b50) {
      offset += 1
      continue
    }
    const fileNameLength = view.getUint16(offset + 28, true)
    const extraLength = view.getUint16(offset + 30, true)
    const commentLength = view.getUint16(offset + 32, true)
    const fileName = decoder.decode(bytes.subarray(offset + 46, offset + 46 + fileNameLength))
    if (fileName === entryName) {
      view.setUint32(offset + 24, uncompressedSize, true)
      return bytes.buffer
    }
    offset += 46 + fileNameLength + extraLength + commentLength
  }

  throw new Error(`Missing central-directory entry: ${entryName}`)
}

async function createForgedDocxZipBomb() {
  const zip = new JSZip()
  zip.file('[Content_Types].xml', `<?xml version="1.0" encoding="UTF-8"?>
    <Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
      <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
    </Types>`)
  zip.file('word/document.xml', `<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
      <w:body><w:p><w:r><w:t>${'A'.repeat(5_000_100)}</w:t></w:r></w:p></w:body>
    </w:document>`)
  const archive = await zip.generateAsync({
    type: 'arraybuffer',
    compression: 'DEFLATE',
    compressionOptions: { level: 1 },
  })
  return forgeCentralDirectorySize(archive, 'word/document.xml', 100)
}

function streamingZipEntry(chunks: Uint8Array[], declaredSize = 1) {
  let paused = false
  let resumed = false
  const entry = {
    dir: false,
    _data: { uncompressedSize: declaredSize },
    internalStream: () => {
      let onData: (chunk: Uint8Array) => void = () => {}
      let onError: (error: unknown) => void = () => {}
      let onEnd = () => {}
      const stream = {
        on(event: 'data' | 'error' | 'end', listener: (value?: unknown) => void) {
          if (event === 'data') onData = listener as (chunk: Uint8Array) => void
          if (event === 'error') onError = listener
          if (event === 'end') onEnd = listener
          return stream
        },
        pause() {
          paused = true
          return stream
        },
        resume() {
          resumed = true
          try {
            for (const chunk of chunks) {
              if (paused) break
              onData(chunk)
            }
            if (!paused) {
              onEnd()
            }
          } catch (error) {
            onError(error)
          }
          return stream
        },
      }
      return stream
    },
  }
  return {
    entry,
    wasPaused: () => paused,
    wasResumed: () => resumed,
  }
}

function mockZip(entries: Record<string, ReturnType<typeof streamingZipEntry>['entry']>) {
  return {
    files: entries,
    file: (path: string) => entries[path] ?? null,
  } as unknown as JSZip
}

function slideXml(paragraphs: string) {
  return `<?xml version="1.0" encoding="UTF-8"?>
    <p:sld
      xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
      xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
      <p:cSld><p:spTree><p:sp><p:txBody>${paragraphs}</p:txBody></p:sp></p:spTree></p:cSld>
    </p:sld>`
}

async function createPptxFixture() {
  const zip = new JSZip()
  zip.file('[Content_Types].xml', `<?xml version="1.0" encoding="UTF-8"?>
    <Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
      <Default Extension="xml" ContentType="application/xml"/>
      <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
      <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
      <Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
    </Types>`)
  zip.file('ppt/presentation.xml', `<?xml version="1.0" encoding="UTF-8"?>
    <p:presentation
      xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
      xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
      <p:sldIdLst>
        <p:sldId id="256" r:id="rId2"/>
        <p:sldId id="257" r:id="rId1"/>
      </p:sldIdLst>
    </p:presentation>`)
  zip.file('ppt/_rels/presentation.xml.rels', `<?xml version="1.0" encoding="UTF-8"?>
    <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
      <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
      <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
    </Relationships>`)
  zip.file('ppt/slides/slide1.xml', slideXml(`
    <a:p><a:r><a:t>Overview</a:t></a:r></a:p>
    <a:p><a:r><a:t>Alpha</a:t></a:r><a:br/><a:r><a:t>Beta</a:t></a:r></a:p>`))
  zip.file('ppt/slides/slide2.xml', slideXml(`
    <a:p><a:r><a:t>Roadmap</a:t></a:r></a:p>
    <a:p><a:r><a:t>R&amp;D priorities</a:t></a:r></a:p>`))
  zip.file('ppt/notesSlides/notesSlide1.xml', `<?xml version="1.0" encoding="UTF-8"?>
    <p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
      xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
      <a:p><a:r><a:t>Speaker secret — do not extract</a:t></a:r></a:p>
    </p:notes>`)
  return zip.generateAsync({ type: 'arraybuffer' })
}

describe('extractFileContext', () => {
  it('extracts UTF-8 CSV text for chat context', async () => {
    const file = new File(['\uFEFFname,status\r\nAlpha,Todo'], 'records.csv', { type: 'text/csv' })
    await expect(extractFileContext(file)).resolves.toBe('name,status\nAlpha,Todo')
  })

  it('extracts XLSX sheets and cells in workbook order', async () => {
    const workbook = XLSX.utils.book_new()
    XLSX.utils.book_append_sheet(
      workbook,
      XLSX.utils.aoa_to_sheet([['Name', 'Status'], ['Alpha', 'Todo']]),
      'Summary',
    )
    XLSX.utils.book_append_sheet(
      workbook,
      XLSX.utils.aoa_to_sheet([['Owner'], ['Takanori']]),
      'Owners',
    )
    const buffer = XLSX.write(workbook, { type: 'array', bookType: 'xlsx' }) as ArrayBuffer
    const file = new File([buffer], 'records.xlsx', {
      type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    })

    const content = await extractFileContext(file)
    expect(content).toContain('# Summary\nName,Status\nAlpha,Todo')
    expect(content).toContain('# Owners\nOwner\nTakanori')
    expect(content.indexOf('# Summary')).toBeLessThan(content.indexOf('# Owners'))
  })

  it('rejects an XLSX worksheet whose declared cell range is too wide to iterate safely', async () => {
    const file = new File(
      [await createXlsxWithWorksheetDimension('A1:XFD1')],
      'wide-range.xlsx',
      { type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' },
    )

    await expect(extractFileContext(file)).rejects.toMatchObject({
      code: 'unreadable-file',
    })
  })

  it('rejects an XLSX workbook with too many sheets', async () => {
    const workbook = XLSX.utils.book_new()
    for (let index = 0; index < 51; index += 1) {
      XLSX.utils.book_append_sheet(
        workbook,
        XLSX.utils.aoa_to_sheet([[index]]),
        `Sheet${index + 1}`,
      )
    }
    const buffer = XLSX.write(workbook, { type: 'array', bookType: 'xlsx' }) as ArrayBuffer
    const file = new File([buffer], 'too-many-sheets.xlsx', {
      type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    })

    await expect(extractFileContext(file)).rejects.toMatchObject({
      code: 'unreadable-file',
    })
  })

  it('extracts DOCX paragraph text in document order', async () => {
    const file = new File([await createDocxFixture()], 'brief.docx', {
      type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    })

    const content = await extractFileContext(file)
    expect(content).toContain('Project brief')
    expect(content).toContain('Ship the client safely')
    expect(content.indexOf('Project brief')).toBeLessThan(content.indexOf('Ship the client safely'))
  })

  it('extracts PDF page text from a real PDF document', async () => {
    const file = new File([createPdfFixture()], 'report.pdf', { type: 'application/pdf' })

    const content = await extractFileContext(file)
    expect(content).toContain('## Page 1')
    expect(content).toContain('Quarterly report')
    expect(content).toContain('Revenue up')
  })

  it('stops PDF extraction at the page cap and marks the context as truncated', async () => {
    const pages = Array.from({ length: 201 }, (_, index) => [`Page ${index + 1}`])
    const file = new File([createPdfDocument(pages)], 'many-pages.pdf', {
      type: 'application/pdf',
    })

    const content = await extractFileContext(file, 100_000)
    expect(content).toContain('## Page 200\nPage 200')
    expect(content).not.toContain('## Page 201')
    expect(content).toContain('[Content truncated]')
  })

  it('stops accumulating PDF text once the requested context budget is reached', async () => {
    const file = new File(
      [createPdfDocument([Array.from({ length: 1_000 }, () => 'A'.repeat(20))])],
      'long-page.pdf',
      { type: 'application/pdf' },
    )

    const content = await extractFileContext(file, 100)
    expect(content.length).toBeLessThanOrEqual(121)
    expect(content.length).toBeGreaterThan(100)
    expect(content.endsWith('[Content truncated]')).toBe(true)
  })

  it('configures the standard PDF.js worker URL in a browser runtime', () => {
    const pdfjsLib = { GlobalWorkerOptions: { workerSrc: '' } }
    vi.stubGlobal('process', undefined)

    try {
      configurePdfJsWorker(pdfjsLib)
      expect(pdfjsLib.GlobalWorkerOptions.workerSrc).toBe(PDF_WORKER_URL)
    } finally {
      vi.unstubAllGlobals()
    }
  })

  it.each([
    ['DOCX', 'oversized.docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', 'word/document.xml'],
    ['PPTX', 'oversized.pptx', 'application/vnd.openxmlformats-officedocument.presentationml.presentation', 'ppt/presentation.xml'],
  ])('rejects oversized %s XML before decompressing it', async (
    _label,
    fileName,
    mimeType,
    oversizedPath,
  ) => {
    const decompress = vi.fn()
    const loadAsync = vi.spyOn(JSZip, 'loadAsync').mockResolvedValue({
      file: (path: string) => {
        if (path === '[Content_Types].xml') return { dir: false }
        if (path === oversizedPath) {
          return {
            dir: false,
            _data: { uncompressedSize: 5_000_001 },
            async: decompress,
          }
        }
        return null
      },
    } as unknown as JSZip)

    try {
      const file = new File([new Uint8Array([0x50, 0x4b, 0x03, 0x04])], fileName, {
        type: mimeType,
      })
      await expect(extractFileContext(file)).rejects.toMatchObject({
        code: 'unreadable-file',
      })
      expect(decompress).not.toHaveBeenCalled()
    } finally {
      loadAsync.mockRestore()
    }
  })

  it('stops decompression when a ZIP entry understates its expanded size', async () => {
    const archive = await createForgedDocxZipBomb()
    expect(archive.byteLength).toBeLessThan(50_000)
    const file = new File([archive], 'forged.docx', {
      type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    })

    await expect(extractFileContext(file)).rejects.toMatchObject({
      code: 'unreadable-file',
    })
  })

  it('shares the actual XML expansion budget across all PPTX slides', async () => {
    const encode = (value: string) => Uint8Array.from(new TextEncoder().encode(value))
    const presentation = streamingZipEntry([encode(`<?xml version="1.0"?>
      <p:presentation
        xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
        <p:sldIdLst><p:sldId r:id="rId1"/><p:sldId r:id="rId2"/></p:sldIdLst>
      </p:presentation>`)])
    const relationships = streamingZipEntry([encode(`<?xml version="1.0"?>
      <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
        <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
        <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
      </Relationships>`)])
    const firstSlide = streamingZipEntry([
      encode(slideXml(`<a:p><a:r><a:t>${'A'.repeat(2_600_000)}</a:t></a:r></a:p>`)),
    ])
    const secondSlide = streamingZipEntry([
      encode(slideXml(`<a:p><a:r><a:t>${'B'.repeat(2_600_000)}</a:t></a:r></a:p>`)),
    ])
    const contentTypes = streamingZipEntry([encode('<Types/>')])
    const loadAsync = vi.spyOn(JSZip, 'loadAsync').mockResolvedValue(mockZip({
      '[Content_Types].xml': contentTypes.entry,
      'ppt/presentation.xml': presentation.entry,
      'ppt/_rels/presentation.xml.rels': relationships.entry,
      'ppt/slides/slide1.xml': firstSlide.entry,
      'ppt/slides/slide2.xml': secondSlide.entry,
    }))

    try {
      const file = new File([new Uint8Array([0x50, 0x4b, 0x03, 0x04])], 'bomb.pptx', {
        type: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
      })
      await expect(extractFileContext(file)).rejects.toMatchObject({
        code: 'unreadable-file',
      })
      expect(firstSlide.wasResumed()).toBe(true)
      expect(secondSlide.wasResumed()).toBe(true)
      expect(secondSlide.wasPaused()).toBe(true)
    } finally {
      loadAsync.mockRestore()
    }
  })

  it('streams every XLSX entry through an aggregate budget despite understated metadata', async () => {
    const smallEntry = streamingZipEntry([new Uint8Array([1])])
    const expandedChunk = new Uint8Array(1024 * 1024)
    const oversizedSheet = streamingZipEntry(Array(21).fill(expandedChunk))
    const loadAsync = vi.spyOn(JSZip, 'loadAsync').mockResolvedValue(mockZip({
      '[Content_Types].xml': smallEntry.entry,
      'xl/workbook.xml': streamingZipEntry([new Uint8Array([2])]).entry,
      'xl/worksheets/sheet1.xml': oversizedSheet.entry,
    }))

    try {
      const file = new File([new Uint8Array([0x50, 0x4b, 0x03, 0x04])], 'bomb.xlsx', {
        type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      })
      await expect(extractFileContext(file)).rejects.toMatchObject({
        code: 'unreadable-file',
      })
      expect(oversizedSheet.wasPaused()).toBe(true)
    } finally {
      loadAsync.mockRestore()
    }
  })

  it('extracts PPTX slide body in presentation order and excludes speaker notes', async () => {
    const file = new File([await createPptxFixture()], 'roadmap.pptx', {
      type: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    })

    const content = await extractFileContext(file)
    expect(content).toContain('## Slide 1\nRoadmap\nR&D priorities')
    expect(content).toContain('## Slide 2\nOverview\nAlpha\nBeta')
    expect(content.indexOf('Roadmap')).toBeLessThan(content.indexOf('Overview'))
    expect(content).not.toContain('Speaker secret')
  })

  it('truncates large extracted content', async () => {
    const file = new File(['abcdefghij'], 'records.csv', { type: 'text/csv' })
    await expect(extractFileContext(file, 5)).resolves.toBe('abcde\n\n[Content truncated]')
  })

  it('returns an explicit empty context for unknown file types', async () => {
    const file = new File(['opaque'], 'archive.bin', { type: 'application/octet-stream' })
    await expect(extractFileContext(file)).resolves.toBe('')
  })

  it.each([
    ['damaged PDF', new File(['not a pdf'], 'broken.pdf', { type: 'application/pdf' })],
    ['damaged DOCX', new File(['not a zip'], 'broken.docx', {
      type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    })],
    ['encrypted or compound-wrapped PPTX', new File([
      new Uint8Array([0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]),
    ], 'encrypted.pptx', {
      type: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    })],
    ['invalid UTF-8 CSV', new File([new Uint8Array([0xff, 0xfe, 0xfd])], 'broken.csv', {
      type: 'text/csv',
    })],
  ])('fails safely without invented context for %s', async (_label, file) => {
    const extraction = extractFileContext(file)
    await expect(extraction).rejects.toBeInstanceOf(FileContextExtractionError)
    await expect(extraction).rejects.toMatchObject({
      code: 'unreadable-file',
      name: 'FileContextExtractionError',
    })
  })
})
