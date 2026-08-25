/**
 * Picks the organizations that belong on the dashboard.
 *
 * Most organizations are created through Library and sit on the Library
 * platform, so matching `platformTenantId` finds them. An organization adopted
 * with `seedLibraryTenant` is different: that mutation registers the tenant
 * with Library but leaves it on its original Tachyon platform, so the platform
 * check alone would hide it. `libraryTenantIds` carries the tenants that
 * `accessibleTenants` reported as already having a Library organization, which
 * covers exactly that case.
 */
export function selectVisibleOrgs<
	T extends { id: string; platformTenantId: string },
>(orgs: T[], libraryTenantIds: Set<string>, platformId: string): T[] {
	return orgs.filter(
		org => org.platformTenantId === platformId || libraryTenantIds.has(org.id),
	)
}
