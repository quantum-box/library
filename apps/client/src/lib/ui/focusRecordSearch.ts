const RECORD_SEARCH_SELECTOR = [
  '[data-testid="library-table-global-filter"]',
  '[data-testid="records-global-filter"]',
].join(', ')

/** Focus the search field for either repository-backed or local data tables. */
export function focusRecordSearchInput(root: ParentNode = document): boolean {
  const input = root.querySelector<HTMLInputElement>(RECORD_SEARCH_SELECTOR)
  if (!input) return false

  input.focus()
  input.select()
  return true
}

interface FocusRecordSearchOptions {
  root?: ParentNode
  attempts?: number
  delayMs?: number
}

/**
 * Wait for a table route to finish rendering, then focus its record search.
 * Route navigation can resolve before the input is committed to the DOM, so a
 * bounded retry keeps the global shortcut deterministic without an arbitrary
 * one-shot delay.
 */
export async function focusRecordSearchInputWhenReady({
  root = document,
  attempts = 20,
  delayMs = 50,
}: FocusRecordSearchOptions = {}): Promise<boolean> {
  const maxAttempts = Math.max(1, attempts)
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    if (focusRecordSearchInput(root)) return true
    if (attempt < maxAttempts - 1) {
      await new Promise<void>((resolve) => window.setTimeout(resolve, delayMs))
    }
  }
  return false
}
