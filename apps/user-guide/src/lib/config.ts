/**
 * Base URL of the Library API the guide describes.
 *
 * Every example on the site is built from this, so a guide served beside a
 * preview deployment documents that deployment rather than production.
 */
export const apiBaseUrl: string =
  import.meta.env.VITE_LIBRARY_API_BASE_URL || 'http://localhost:50053'

/** Organization used in examples until the reader substitutes their own. */
export const sampleOrg = 'acme'

/** Repository used in examples until the reader substitutes their own. */
export const sampleRepo = 'handbook'

/** Where the reader issues a key, which is a page in the web app, not here. */
export const portalUrl: string =
  import.meta.env.VITE_LIBRARY_WEB_URL || 'https://library.txcloud.app'
