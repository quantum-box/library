/**
 * Bridge to the native macOS menu bar built in `src-tauri/src/macos_menu.rs`.
 *
 * macOS localizes nothing in that bar: the standard `Services` / `Quit …` /
 * `Edit` entries are English literals in Tauri's menu library, exactly like
 * the two entries the app owns. The Rust side therefore lays the bar out in
 * English and waits for the labels below, which is also why the language
 * switcher can retitle it without a relaunch.
 */
import { invoke } from '@tauri-apps/api/core'
import type { I18nContextValue } from '../../i18n'

/** Field for field, `MenuLabels` in `src-tauri/src/macos_menu.rs`. */
export interface MenuLabels {
  about: string
  checkForUpdates: string
  services: string
  hide: string
  hideOthers: string
  quit: string
  file: string
  newTab: string
  closeWindow: string
  edit: string
  undo: string
  redo: string
  cut: string
  copy: string
  paste: string
  selectAll: string
  view: string
  toggleFullScreen: string
  window: string
  minimize: string
  zoom: string
  help: string
}

/**
 * `productName` fills the `{name}` in `About …` / `Hide …` / `Quit …`. It is
 * the product name, so it is the one part of the bar that never translates.
 */
export function menuLabels(
  t: I18nContextValue['t'],
  productName: string,
): MenuLabels {
  return {
    about: t('menuBar.about', { name: productName }),
    // macOS marks any menu item that opens a dialog with an ellipsis. The
    // shared `account.checkForUpdates` string also names the in-app account
    // menu entry, where that convention does not apply, so the ellipsis is
    // added here rather than baked into all eleven catalogs.
    checkForUpdates: `${t('account.checkForUpdates')}…`,
    services: t('menuBar.services'),
    hide: t('menuBar.hide', { name: productName }),
    hideOthers: t('menuBar.hideOthers'),
    quit: t('menuBar.quit', { name: productName }),
    file: t('menuBar.file'),
    newTab: t('menuBar.newTab'),
    closeWindow: t('menuBar.closeWindow'),
    edit: t('menuBar.edit'),
    undo: t('menuBar.undo'),
    redo: t('menuBar.redo'),
    cut: t('menuBar.cut'),
    copy: t('menuBar.copy'),
    paste: t('menuBar.paste'),
    selectAll: t('menuBar.selectAll'),
    view: t('menuBar.view'),
    toggleFullScreen: t('menuBar.toggleFullScreen'),
    window: t('menuBar.window'),
    minimize: t('menuBar.minimize'),
    zoom: t('menuBar.zoom'),
    help: t('menuBar.help'),
  }
}

export function fetchProductName() {
  return invoke<string>('app_product_name')
}

export function pushMenuLabels(labels: MenuLabels) {
  return invoke<void>('set_menu_labels', { labels })
}
