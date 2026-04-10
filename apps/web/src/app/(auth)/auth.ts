// Stub: replaces the deleted Next.js auth module.
// In the Vite SPA, authentication is handled via the Cognito auth context (`@/auth`).

/**
 * @deprecated Migration stub. Use `useAuth()` from `@/auth` instead.
 */
export async function auth(): Promise<{ user: { accessToken: string } } | null> {
  // This stub is only here so files that import `auth()` can compile.
  // At runtime these server-action files should not be called;
  // the SPA equivalents use `useAuth()` and call the API client directly.
  console.warn('[migration stub] auth() called – this should be replaced with useAuth()')
  return null
}
