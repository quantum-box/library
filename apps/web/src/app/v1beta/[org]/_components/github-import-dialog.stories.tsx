import type { Meta, StoryObj } from '@storybook/react'
import { expect, fn, userEvent, within } from '@storybook/test'
import { GitHubImportDialog } from './github-import-dialog'

const meta = {
	title: 'v1beta/Organization/GitHubImportDialog',
	component: GitHubImportDialog,
	parameters: {
		layout: 'centered',
		nextjs: {
			appDirectory: true,
		},
	},
	tags: ['autodocs', 'github-import'],
	decorators: [
		Story => (
			<div className='min-h-[600px] flex items-center justify-center p-8'>
				<Story />
			</div>
		),
	],
	args: {
		onImportComplete: fn(),
	},
} satisfies Meta<typeof GitHubImportDialog>

export default meta
type Story = StoryObj<typeof meta>

/**
 * Default state showing the import button
 */
export const Default: Story = {
	args: {
		org: 'acme-corp',
		existingRepos: [],
	},
}

/**
 * With existing repositories that will trigger warning
 */
export const WithExistingRepos: Story = {
	args: {
		org: 'acme-corp',
		existingRepos: [
			{ username: 'docs' },
			{ username: 'product-catalog' },
			{ username: 'knowledge-base' },
		],
	},
}

/**
 * Dialog opened state - shows the repository selection step
 * Note: Play function is skipped in tests because it triggers server action calls
 */
export const DialogOpened: Story = {
	args: {
		org: 'acme-corp',
		existingRepos: [],
	},
	parameters: {
		// Skip test because dialog open triggers server action (listGitHubRepositories)
		test: { skip: true },
	},
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)
		const button = canvas.getByRole('button', { name: /import from github/i })
		await userEvent.click(button)
	},
}

/**
 * Interactive test - verifies button exists and is clickable
 * Note: Does not open dialog to avoid server action calls
 */
export const InteractiveOpen: Story = {
	args: {
		org: 'test-org',
		existingRepos: [{ username: 'existing-repo' }],
	},
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify import button exists
		const importButton = canvas.getByRole('button', {
			name: /import from github/i,
		})
		await expect(importButton).toBeInTheDocument()
		await expect(importButton).toBeEnabled()
	},
}

/**
 * Multiple existing repos - demonstrates duplicate warning scenario
 */
export const ManyExistingRepos: Story = {
	args: {
		org: 'large-org',
		existingRepos: [
			{ username: 'documentation' },
			{ username: 'api-docs' },
			{ username: 'user-guides' },
			{ username: 'tutorials' },
			{ username: 'changelog' },
			{ username: 'release-notes' },
		],
	},
}

/**
 * Empty organization with no existing repos
 */
export const EmptyOrganization: Story = {
	args: {
		org: 'new-org',
		existingRepos: [],
	},
}
