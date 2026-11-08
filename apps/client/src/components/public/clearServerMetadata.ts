import { appKitConfig } from '../../app/kitConfig'

/** Let the app own its head before document-title effects remember a title. */
export function clearServerMetadata(pathname: string) {
  if (pathname === '/public' || pathname.startsWith('/public/')) return

  const title = document.head.querySelector('title[data-public-docs-title]')
  if (title) {
    title.removeAttribute('data-public-docs-title')
    document.title = appKitConfig.app.displayName
  }
  document.head.querySelectorAll('[data-public-docs-meta]').forEach((node) => node.remove())
}
