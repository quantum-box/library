import pdfWorkerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url'

interface PdfJsWorkerConfiguration {
  GlobalWorkerOptions: {
    workerSrc: string
  }
}

export const PDF_WORKER_URL = pdfWorkerUrl

function isNodeRuntime() {
  return typeof process !== 'undefined' && Boolean(process.versions?.node)
}

export function configurePdfJsWorker(pdfjsLib: PdfJsWorkerConfiguration) {
  if (typeof window !== 'undefined' && !isNodeRuntime()) {
    pdfjsLib.GlobalWorkerOptions.workerSrc = PDF_WORKER_URL
  }
}

export async function loadPdfJs() {
  if (isNodeRuntime()) {
    return import(/* @vite-ignore */ 'pdfjs-dist/legacy/build/pdf.mjs')
  }
  const pdfjsLib = await import('pdfjs-dist')
  configurePdfJsWorker(pdfjsLib)
  return pdfjsLib
}
