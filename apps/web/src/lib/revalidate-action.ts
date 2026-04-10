// No-op in SPA mode - no server-side revalidation
export function revalidateAction(_path: string) {
  // In SPA mode, use SWR mutate or router invalidation instead
}
