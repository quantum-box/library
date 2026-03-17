import { DefaultRole } from '@/gen/graphql'
import type { Meta, StoryObj } from '@storybook/react'
import { fn } from '@storybook/test'
import { OrganizationPageUi } from './organization-page-ui'

const meta = {
	title: 'v1beta/Organization/OrganizationPageUi',
	component: OrganizationPageUi,
	parameters: {
		layout: 'fullscreen',
		nextjs: {
			appDirectory: true,
		},
	},
	tags: ['autodocs', 'organization'],
	decorators: [
		Story => (
			<div className='min-h-screen bg-background'>
				<Story />
			</div>
		),
	],
	args: {
		onSubmit: fn(),
		hasLinearConnection: false,
		tenantId: 'tn_01exampletenantid0000000000000',
	},
} satisfies Meta<typeof OrganizationPageUi>

export default meta
type Story = StoryObj<typeof meta>

const mockOrganization = {
	name: 'Acme Corporation',
	username: 'acme-corp',
	description: 'Building the future of data management',
	website: 'https://acme.example.com',
	repos: [
		{
			id: 'repo-1',
			username: 'product-catalog',
			description:
				'Central repository for all product data and catalog management',
			isPublic: true,

			stars: 24,
			updatedAt: '2026-03-15T10:00:00Z',
		},
		{
			id: 'repo-2',
			username: 'customer-data',
			description: 'Customer relationship and analytics data',
			isPublic: false,

			stars: 8,
			updatedAt: '2026-03-10T14:30:00Z',
		},
		{
			id: 'repo-3',
			username: 'analytics-warehouse',
			description: 'Business intelligence and reporting data warehouse',
			isPublic: false,

			stars: 3,
			updatedAt: '2026-02-28T09:00:00Z',
		},
	],
	users: [
		{
			id: 'user-1',
			name: 'John Doe',
			image: null,
			email: 'john.doe@acme.com',
			role: DefaultRole.Owner,
		},
		{
			id: 'user-2',
			name: 'Jane Smith',
			image: null,
			email: 'jane.smith@acme.com',
			role: DefaultRole.Manager,
		},
		{
			id: 'user-3',
			name: 'Bob Wilson',
			image: null,
			email: 'bob.wilson@acme.com',
			role: DefaultRole.General,
		},
	],
}

/**
 * Default view showing the repositories tab with full data
 */
export const Default: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'repositories',
		isViewOnly: false,
		organization: mockOrganization,
	},
}

/**
 * View-only mode (unauthenticated user)
 * Shows limited tabs and no action buttons
 */
export const ViewOnly: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'repositories',
		isViewOnly: true,
		organization: mockOrganization,
	},
}

/**
 * Organization with no website set
 */
export const NoWebsite: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'repositories',
		isViewOnly: false,
		organization: {
			...mockOrganization,
			website: null,
		},
	},
}

/**
 * Empty organization with no repositories
 */
export const EmptyRepositories: Story = {
	args: {
		org: 'new-org',
		activeTab: 'repositories',
		isViewOnly: false,
		organization: {
			...mockOrganization,
			repos: [],
		},
	},
}

/**
 * Activity tab view
 */
export const ActivityTab: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'activity',
		isViewOnly: false,
		organization: mockOrganization,
	},
}

/**
 * Insights tab view
 */
export const InsightsTab: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'insights',
		isViewOnly: false,
		organization: mockOrganization,
	},
}

/**
 * Members tab view showing the team table
 */
export const MembersTab: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'members',
		isViewOnly: false,
		organization: mockOrganization,
	},
}

/**
 * Settings tab view with organization form
 */
export const SettingsTab: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'settings',
		isViewOnly: false,
		organization: mockOrganization,
	},
}

/**
 * Settings tab with API key list slot
 */
export const SettingsTabWithApiKeys: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'settings',
		isViewOnly: false,
		organization: mockOrganization,
		apiKeyListSlot: (
			<div className='p-4 border rounded-lg bg-muted'>
				<h3 className='font-semibold mb-2'>API Keys</h3>
				<p className='text-sm text-muted-foreground'>
					API key management component placeholder
				</p>
			</div>
		),
	},
}

/**
 * Organization with a single member
 */
export const SingleMember: Story = {
	args: {
		org: 'solo-org',
		activeTab: 'repositories',
		isViewOnly: false,
		organization: {
			...mockOrganization,
			users: [mockOrganization.users[0]],
		},
	},
}

/**
 * Organization with many repositories
 */
export const ManyRepositories: Story = {
	args: {
		org: 'large-org',
		activeTab: 'repositories',
		isViewOnly: false,
		organization: {
			...mockOrganization,
			repos: [
				...mockOrganization.repos,
				{
					id: 'repo-4',
					username: 'inventory-system',
					description: 'Real-time inventory tracking and management',
					isPublic: true,
		
					stars: 42,
					updatedAt: '2026-03-16T08:00:00Z',
				},
				{
					id: 'repo-5',
					username: 'order-processing',
					description: 'Order fulfillment and processing pipeline',
					isPublic: false,
		
					stars: 12,
					updatedAt: '2026-03-14T16:00:00Z',
				},
				{
					id: 'repo-6',
					username: 'supplier-data',
					description: 'Supplier relationship and procurement data',
					isPublic: false,
		
					stars: 5,
					updatedAt: '2026-03-01T12:00:00Z',
				},
				{
					id: 'repo-7',
					username: 'financial-reports',
					description: 'Financial reporting and compliance data',
					isPublic: false,
		
					stars: 1,
					updatedAt: '2026-02-15T10:00:00Z',
				},
			],
		},
	},
}

/**
 * Mobile viewport - tabs should be horizontally scrollable
 */
export const Mobile: Story = {
	args: {
		org: 'acme-corp',
		activeTab: 'repositories',
		isViewOnly: false,
		organization: mockOrganization,
	},
	parameters: {
		viewport: {
			defaultViewport: 'mobile1',
		},
	},
}

/**
 * Organization with no description
 */
export const NoDescription: Story = {
	args: {
		org: 'minimal-org',
		activeTab: 'repositories',
		isViewOnly: false,
		organization: {
			...mockOrganization,
			description: null,
		},
	},
}
