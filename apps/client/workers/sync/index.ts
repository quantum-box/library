/**
 * Photon sync edge — Engine proxy and Live Durable Object relay.
 *
 * The implementation is `@quantum-box/photon/worker`. This file exists to name
 * it as this app's Worker entry point: `wrangler.jsonc` points here, and the
 * Durable Object binding resolves `PhotonSyncRoom` from the entry module.
 *
 * This used to be a vendored copy of that file, taken at import and never
 * updated. By the time it was replaced it was missing two things the upstream
 * worker had gained: the caller's own bearer token being forwarded to the cloud
 * Engine (which authorizes per tenant, so a user token carries a narrower grant
 * than the edge's service token), and the `engine-changed` frame that wakes
 * other clients' sync loops right after a push instead of leaving them to their
 * poll interval.
 */

export { PhotonSyncRoom, default } from '@quantum-box/photon/worker'
export type { Env } from '@quantum-box/photon/worker'
