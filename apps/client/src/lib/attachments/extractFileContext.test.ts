import JSZip from 'jszip'
import * as XLSX from 'xlsx'
import { describe, expect, it } from 'vitest'
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

function createPdfFixture() {
  const stream = [
    'BT',
    '/F1 18 Tf',
    '72 720 Td',
    '(Quarterly report) Tj',
    '0 -24 Td',
    '(Revenue up) Tj',
    'ET',
  ].join('\n')
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
    `<< /Length ${stream.length} >>\nstream\n${stream}\nendstream`,
  ]

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
