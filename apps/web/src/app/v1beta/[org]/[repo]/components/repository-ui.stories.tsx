import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryObj } from '@storybook/react'
import { expect, userEvent, waitFor, within } from '@storybook/test'
import V1BetaLayout from '../layout.storybook'
import { RepositoryUi } from './repository-ui'

const meta = {
	title: 'V1Beta/RepositoryUi',
	component: RepositoryUi,
	decorators: [
		Story => (
			<V1BetaLayout params={{ org: 'demo-org', repo: 'demo-repo' }}>
				<Story />
			</V1BetaLayout>
		),
	],
	parameters: {
		layout: 'fullscreen',
		nextjs: {
			navigation: {
				segments: [
					['org', 'demo-org'],
					['repo', 'demo-repo'],
				],
				pathname: '/v1beta/demo-org/demo-repo',
			},
		},
	},
} satisfies Meta<typeof RepositoryUi>

export default meta

type Story = StoryObj<typeof RepositoryUi>

export const Default: Story = {
	args: {
		org: 'demo-org',
		repo: 'demo-repo',
		repoName: 'Demo Repository',
		isPublic: true,
		onMetaUpdate: async () => {},
		dataList: {
			items: [
				{
					id: 'data-1',
					name: 'Chapter 1',
					createdAt: '2024-04-01T00:00:00Z',
					updatedAt: '2024-04-05T00:00:00Z',
					propertyData: [],
				},
				{
					id: 'data-2',
					name: 'Chapter 2',
					createdAt: '2024-04-02T00:00:00Z',
					updatedAt: '2024-04-06T00:00:00Z',
					propertyData: [],
				},
				{
					id: 'data-3',
					name: 'Appendix',
					createdAt: '2024-04-03T00:00:00Z',
					updatedAt: '2024-04-07T00:00:00Z',
					propertyData: [],
				},
			],
			paginator: {
				currentPage: 1,
				totalItems: 3,
				itemsPerPage: 20,
				totalPages: 1,
			},
		},
		properties: [
			{
				id: 'property-1',
				name: 'name',
				typ: PropertyType.String,
			},
		],
		about: 'Storybook用のデモデータです。',
		labels: ['demo', 'storybook'],
		url: 'https://library.example.com/repos/demo-repo',
		sources: [
			{
				id: 'source-1',
				name: 'Public Catalog',
				url: 'https://library.example.com/files/catalog.pdf',
				isPrimary: true,
			},
		],
		contributors: [
			{
				userId: 'user-1',
				role: 'Owner',
				name: 'Alice Editor',
			},
			{
				userId: 'user-2',
				role: 'Reviewer',
				name: 'Bob Proofreader',
			},
		],
	},
}

export const Empty: Story = {
	args: {
		org: 'demo-org',
		repo: 'empty-repo',
		repoName: 'Empty Repository',
		isPublic: false,
		onMetaUpdate: async () => {},
		dataList: {
			items: [],
			paginator: {
				currentPage: 1,
				totalItems: 0,
				itemsPerPage: 20,
				totalPages: 1,
			},
		},
		properties: [],
		about: '',
		labels: [],
		url: '',
		sources: [],
		contributors: [],
	},
}

export const WithLocationMapView: Story = {
	args: {
		org: 'demo-org',
		repo: 'locations-repo',
		repoName: 'Locations Repository',
		isPublic: true,
		onMetaUpdate: async () => {},
		dataList: {
			items: [
				{
					id: 'loc-1',
					name: 'Tokyo Tower',
					createdAt: '2024-04-01T00:00:00Z',
					updatedAt: '2024-04-05T00:00:00Z',
					propertyData: [
						{
							propertyId: 'location-prop',
							value: {
								__typename: 'LocationValue',
								latitude: 35.6586,
								longitude: 139.7454,
							},
						},
					],
				},
				{
					id: 'loc-2',
					name: 'Osaka Castle',
					createdAt: '2024-04-02T00:00:00Z',
					updatedAt: '2024-04-06T00:00:00Z',
					propertyData: [
						{
							propertyId: 'location-prop',
							value: {
								__typename: 'LocationValue',
								latitude: 34.6873,
								longitude: 135.5262,
							},
						},
					],
				},
				{
					id: 'loc-3',
					name: 'Hiroshima Peace Memorial',
					createdAt: '2024-04-03T00:00:00Z',
					updatedAt: '2024-04-07T00:00:00Z',
					propertyData: [
						{
							propertyId: 'location-prop',
							value: {
								__typename: 'LocationValue',
								latitude: 34.3955,
								longitude: 132.4536,
							},
						},
					],
				},
				{
					id: 'loc-4',
					name: 'Sapporo Clock Tower',
					createdAt: '2024-04-04T00:00:00Z',
					updatedAt: '2024-04-08T00:00:00Z',
					propertyData: [
						{
							propertyId: 'location-prop',
							value: {
								__typename: 'LocationValue',
								latitude: 43.0625,
								longitude: 141.3544,
							},
						},
					],
				},
				{
					id: 'loc-5',
					name: 'Kyoto Fushimi Inari',
					createdAt: '2024-04-05T00:00:00Z',
					updatedAt: '2024-04-09T00:00:00Z',
					propertyData: [
						{
							propertyId: 'location-prop',
							value: {
								__typename: 'LocationValue',
								latitude: 34.9671,
								longitude: 135.7727,
							},
						},
					],
				},
			],
			paginator: {
				currentPage: 1,
				totalItems: 5,
				itemsPerPage: 20,
				totalPages: 1,
			},
		},
		properties: [
			{
				id: 'name-prop',
				name: 'name',
				typ: PropertyType.String,
			},
			{
				id: 'location-prop',
				name: 'Location',
				typ: PropertyType.Location,
			},
		],
		about:
			'Famous landmarks in Japan with location data. Click the Map tab to see all locations plotted on a map.',
		labels: ['japan', 'landmarks', 'locations'],
		url: '',
		sources: [],
		contributors: [
			{
				userId: 'user-1',
				role: 'Owner',
				name: 'Map Creator',
			},
		],
	},
}

/**
 * Interactive test: Switch between List, Table, and Map views
 */
export const SwitchToMapView: Story = {
	args: WithLocationMapView.args,
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify List/Table/Map tabs are visible
		await waitFor(() => {
			const listTab = canvas.getByRole('tab', { name: /list/i })
			const tableTab = canvas.getByRole('tab', { name: /table/i })
			const mapTab = canvas.getByRole('tab', { name: /map/i })
			expect(listTab).toBeInTheDocument()
			expect(tableTab).toBeInTheDocument()
			expect(mapTab).toBeInTheDocument()
		})

		// Verify initial list view shows data items
		await waitFor(() => {
			expect(canvas.getByText('Tokyo Tower')).toBeInTheDocument()
			expect(canvas.getByText('Osaka Castle')).toBeInTheDocument()
		})

		// Click Map tab to switch to map view
		const mapTab = canvas.getByRole('tab', { name: /map/i })
		await userEvent.click(mapTab)

		// Verify map is displayed
		await waitFor(
			() => {
				// Google Map renders as a region with name "Map"
				const mapRegion = canvas.queryByRole('region', { name: 'Map' })
				// Or fallback placeholder if no API key
				const fallbackText = canvas.queryByText(/locations plotted/i)
				expect(mapRegion || fallbackText).toBeTruthy()
			},
			{ timeout: 5000 },
		)

		// Switch back to List view
		const listTab = canvas.getByRole('tab', { name: /list/i })
		await userEvent.click(listTab)

		// Verify list view is restored
		await waitFor(() => {
			expect(canvas.getByText('Tokyo Tower')).toBeInTheDocument()
		})
	},
}

/**
 * Interactive test: Verify List and Table tabs are shown, but Map tab is hidden when no location data
 */
export const NoMapTabWithoutLocationData: Story = {
	args: Default.args,
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify List and Table tabs are visible, but Map tab is NOT visible
		await waitFor(() => {
			const listTab = canvas.queryByRole('tab', { name: /list/i })
			const tableTab = canvas.queryByRole('tab', { name: /table/i })
			const mapTab = canvas.queryByRole('tab', { name: /map/i })
			expect(listTab).toBeInTheDocument()
			expect(tableTab).toBeInTheDocument()
			expect(mapTab).not.toBeInTheDocument()
		})

		// Verify data list is still displayed
		await waitFor(() => {
			expect(canvas.getByText('Chapter 1')).toBeInTheDocument()
		})
	},
}

/**
 * Interactive test: Switch between List, Table, and Map views
 */
export const SwitchToTableView: Story = {
	args: WithLocationMapView.args,
	play: async ({ canvasElement }) => {
		const canvas = within(canvasElement)

		// Verify all three tabs are visible
		await waitFor(() => {
			const listTab = canvas.getByRole('tab', { name: /list/i })
			const tableTab = canvas.getByRole('tab', { name: /table/i })
			const mapTab = canvas.getByRole('tab', { name: /map/i })
			expect(listTab).toBeInTheDocument()
			expect(tableTab).toBeInTheDocument()
			expect(mapTab).toBeInTheDocument()
		})

		// Verify initial list view shows data items
		await waitFor(() => {
			expect(canvas.getByText('Tokyo Tower')).toBeInTheDocument()
		})

		// Click Table tab to switch to table view
		const tableTab = canvas.getByRole('tab', { name: /table/i })
		await userEvent.click(tableTab)

		// Verify table is displayed with data
		await waitFor(() => {
			// Table should show data - look for table element
			const table = canvas.getByRole('table')
			expect(table).toBeInTheDocument()
			// Data should still be accessible via links
			expect(canvas.getByText('Tokyo Tower')).toBeInTheDocument()
		})

		// Switch back to List view
		const listTab = canvas.getByRole('tab', { name: /list/i })
		await userEvent.click(listTab)

		// Verify list view is restored
		await waitFor(() => {
			expect(canvas.getByText('Tokyo Tower')).toBeInTheDocument()
		})
	},
}
