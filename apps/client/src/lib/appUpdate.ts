import type { Update } from '@tauri-apps/plugin-updater'

export const CHECK_FOR_UPDATES_EVENT = 'library-check-for-updates'

export function requestUpdateCheck() {
  window.dispatchEvent(new CustomEvent(CHECK_FOR_UPDATES_EVENT))
}

// The updater plugin is registered on desktop targets only, so the web build and
// the iOS/Android builds must never call it. The Tauri CLI exports the target
// platform to the frontend build, which `envPrefix` in vite.config.ts passes
// through — an unset value means this is the plain web build.
const desktopPlatforms = ['windows', 'darwin', 'linux']

export function isDesktopApp() {
  return desktopPlatforms.includes(import.meta.env.TAURI_ENV_PLATFORM)
}

export async function checkForAppUpdate(): Promise<Update | null> {
  if (!isDesktopApp()) return null
  const { check } = await import('@tauri-apps/plugin-updater')
  return await check()
}

export type UpdateDownloadProgress = {
  downloaded: number
  total: number | null
}

export async function installAppUpdate(
  update: Update,
  onProgress?: (progress: UpdateDownloadProgress) => void,
) {
  let downloaded = 0
  let total: number | null = null

  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case 'Started':
        total = event.data.contentLength ?? null
        break
      case 'Progress':
        downloaded += event.data.chunkLength
        break
      case 'Finished':
        downloaded = total ?? downloaded
        break
    }
    onProgress?.({ downloaded, total })
  })

  const { relaunch } = await import('@tauri-apps/plugin-process')
  await relaunch()
}
