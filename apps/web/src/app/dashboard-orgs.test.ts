import { describe, expect, it } from 'vitest'
import { selectVisibleOrgs } from './dashboard-orgs'

const LIBRARY_PLATFORM = 'tn_library'
const OTHER_PLATFORM = 'tn_other'

const nativeOrg = { id: 'tn_native', platformTenantId: LIBRARY_PLATFORM }
const importedOrg = { id: 'tn_imported', platformTenantId: OTHER_PLATFORM }
const unrelatedOrg = { id: 'tn_unrelated', platformTenantId: OTHER_PLATFORM }

describe('selectVisibleOrgs', () => {
	it('keeps organizations created on the Library platform', () => {
		expect(selectVisibleOrgs([nativeOrg], new Set(), LIBRARY_PLATFORM)).toEqual(
			[nativeOrg],
		)
	})

	it('keeps an imported tenant even though it stayed on its own platform', () => {
		expect(
			selectVisibleOrgs(
				[importedOrg],
				new Set(['tn_imported']),
				LIBRARY_PLATFORM,
			),
		).toEqual([importedOrg])
	})

	it('drops tenants that have no Library organization', () => {
		expect(
			selectVisibleOrgs([unrelatedOrg], new Set(), LIBRARY_PLATFORM),
		).toEqual([])
	})

	it('returns both kinds together without duplicating either', () => {
		expect(
			selectVisibleOrgs(
				[nativeOrg, importedOrg, unrelatedOrg],
				new Set(['tn_imported']),
				LIBRARY_PLATFORM,
			),
		).toEqual([nativeOrg, importedOrg])
	})

	it('handles an empty organization list', () => {
		expect(
			selectVisibleOrgs([], new Set(['tn_imported']), LIBRARY_PLATFORM),
		).toEqual([])
	})
})
