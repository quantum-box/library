// Stub: replaces the deleted Next.js server action module.
// In the Vite SPA, authentication is handled via the Cognito auth context.

/**
 * @deprecated This is a migration stub. Use `useAuth()` from `@/auth` instead.
 */
export async function loginAction() {
  // In the SPA, login is handled by navigating to /sign_in
  window.location.href = '/sign_in'
}

/**
 * @deprecated This is a migration stub. Use `useAuth()` from `@/auth` instead.
 */
export async function signOutAction() {
  window.location.href = '/sign_out'
}
