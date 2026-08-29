import { useCallback, useEffect, useState } from 'react'
import { useRouterState } from '@tanstack/react-router'
import { MacOSWindowTabs, isDesktopWindowTabOpenClick } from '@tachyon-sdk/native-ui'
import {
  activateWindowTab,
  closeWindowTab,
  createWindowTab,
  fetchTargetOs,
  listWindowTabs,
  listenWindowTabsChanged,
  markWindowTabContentReady,
  tabTitleForPath,
  updateWindowTabTitle,
  type WindowTab,
} from '../../lib/desktop/windowTabs'

/**
 * True only inside the macOS desktop shell. Web, Windows, Linux, and mobile
 * keep the plain layout, so the strip renders nothing there.
 */
function useMacosDesktopShell() {
  const [enabled, setEnabled] = useState(false)

  useEffect(() => {
    let disposed = false
    fetchTargetOs()
      .then((target) => {
        if (!disposed) setEnabled(target === 'macos')
      })
      .catch(console.error)
    return () => {
      disposed = true
    }
  }, [])

  return enabled
}

/**
 * Turns ⌘-click (Ctrl-click off macOS) on an in-app link into a background tab.
 * Without this the WebView would follow its default and open a second window.
 * The listener runs in the capture phase so it wins over the router's own link
 * handling.
 */
function useModifierClickOpensTab(enabled: boolean) {
  useEffect(() => {
    if (!enabled) return

    const onClick = (event: MouseEvent) => {
      if (!isDesktopWindowTabOpenClick(event)) return

      const target = event.target
      if (!(target instanceof Element)) return
      const anchor = target.closest('a[href]')
      if (!(anchor instanceof HTMLAnchorElement)) return
      if (anchor.target && anchor.target !== '_self') return
      if (anchor.hasAttribute('download')) return

      const href = anchor.getAttribute('href')
      if (!href?.startsWith('/') || href.startsWith('//')) return

      event.preventDefault()
      event.stopPropagation()
      createWindowTab(href, false).catch(console.error)
    }

    document.addEventListener('click', onClick, true)
    return () => document.removeEventListener('click', onClick, true)
  }, [enabled])
}

export function WindowTabStrip() {
  const enabled = useMacosDesktopShell()
  const pathname = useRouterState({ select: (state) => state.location.pathname })
  const [tabs, setTabs] = useState<WindowTab[]>([])

  useModifierClickOpensTab(enabled)

  const refreshTabs = useCallback(async () => {
    const next = await listWindowTabs()
    setTabs((current) => {
      // A tab that is still painting reports no selection yet. Keep showing the
      // previous one instead of briefly rendering a strip with nothing active.
      if (next.some((tab) => tab.selected)) return next
      const selected = current.find((tab) => tab.selected)?.label
      return next.map((tab) => ({ ...tab, selected: tab.label === selected }))
    })
  }, [])

  useEffect(() => {
    if (!enabled) return

    let secondFrame = 0
    const firstFrame = requestAnimationFrame(() => {
      secondFrame = requestAnimationFrame(() => {
        markWindowTabContentReady().catch(console.error)
      })
    })

    return () => {
      cancelAnimationFrame(firstFrame)
      cancelAnimationFrame(secondFrame)
    }
  }, [enabled])

  useEffect(() => {
    if (!enabled) return

    let disposed = false
    let unlisten: (() => void) | undefined
    // Subscribe before the first read so a tab opened while this one was still
    // mounting cannot slip between the two.
    listenWindowTabsChanged(() => {
      refreshTabs().catch(console.error)
    })
      .then((stop) => {
        if (disposed) {
          stop()
          return
        }
        unlisten = stop
        return refreshTabs()
      })
      .catch(console.error)

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [enabled, refreshTabs])

  useEffect(() => {
    if (!enabled) return
    updateWindowTabTitle(tabTitleForPath(pathname))
      .then(refreshTabs)
      .catch(console.error)
  }, [enabled, pathname, refreshTabs])

  // Render as soon as the shell is known to be macOS, before the first tab list
  // arrives. The window has no titlebar of its own, so a late strip would drop
  // the content under the traffic lights and then push it down 38px.
  if (!enabled) return null

  return (
    <MacOSWindowTabs
      data-testid="window-tab-strip"
      tabs={tabs.map((tab) => ({ id: tab.label, title: tab.title }))}
      activeTabId={tabs.find((tab) => tab.selected)?.label ?? ''}
      onTabSelect={(label) => void activateWindowTab(label).catch(console.error)}
      onTabClose={(label) => void closeWindowTab(label).catch(console.error)}
      onNewTab={() => void createWindowTab(null, true).catch(console.error)}
      tabListLabel="Library tabs"
      newTabLabel="新しいタブ"
      closeTabLabel={(tab) => `${tab.title}を閉じる`}
    />
  )
}
