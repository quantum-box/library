export const OPEN_CREATE_DATA_EVENT = 'library-open-create-data'
export const OPEN_COMMAND_PALETTE_EVENT = 'library-open-command-palette'

export function openCreateData() {
  window.dispatchEvent(new CustomEvent(OPEN_CREATE_DATA_EVENT))
}

export function openCommandPalette() {
  window.dispatchEvent(new CustomEvent(OPEN_COMMAND_PALETTE_EVENT))
}
