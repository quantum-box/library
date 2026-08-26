import { describe, expect, it } from 'vitest'
import {
	isExtGithubSyncExplicitlyEnabled,
	normalizeExtGithubEditorState,
} from './ext-github-sync-policy'

describe('isExtGithubSyncExplicitlyEnabled', () => {
	it.each([
		[undefined, false],
		[null, false],
		[false, false],
		[true, true],
	] as const)('maps %s to %s', (enabled, expected) => {
		expect(isExtGithubSyncExplicitlyEnabled(enabled)).toBe(expected)
	})
})

describe('normalizeExtGithubEditorState', () => {
	it('resets an empty value to a disabled editor state', () => {
		expect(normalizeExtGithubEditorState(undefined)).toEqual({
			repo: '',
			path: '',
			ref: 'main',
			enabled: false,
		})
	})

	it('keeps legacy metadata without enabled default-deny', () => {
		expect(
			normalizeExtGithubEditorState({
				repo: 'quantum-box/library',
				path: 'docs/example.md',
			}),
		).toEqual({
			repo: 'quantum-box/library',
			path: 'docs/example.md',
			ref: 'main',
			enabled: false,
		})
	})

	it('keeps an explicit ref and enabled flag', () => {
		expect(
			normalizeExtGithubEditorState({
				repo: 'quantum-box/library',
				path: 'docs/example.md',
				ref: 'develop',
				enabled: true,
			}),
		).toEqual({
			repo: 'quantum-box/library',
			path: 'docs/example.md',
			ref: 'develop',
			enabled: true,
		})
	})

	it('falls back to main for a blank ref', () => {
		expect(
			normalizeExtGithubEditorState({
				repo: 'quantum-box/library',
				path: 'docs/example.md',
				ref: '  ',
			}).ref,
		).toBe('main')
	})
})
