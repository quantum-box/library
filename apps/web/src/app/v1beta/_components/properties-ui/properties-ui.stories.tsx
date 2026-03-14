import { PropertyType } from '@/gen/graphql'
import type { Meta, StoryFn } from '@storybook/react'
import { expect, fn, userEvent, waitFor, within } from '@storybook/test'
import { PropertiesUi } from '.'
import V1BetaLayout from '../../[org]/[repo]/layout.storybook'

export default {
	title: 'V1Beta/PropertiesUi',
	component: PropertiesUi,
	parameters: {
		navigation: {
			segments: [
				['org', 'quanta'],
				['repo', 'book'],
			],
			pathname: '/v1beta/quanta/book/properties',
		},
		test: {
			dangerouslyIgnoreUnhandledErrors: true,
		},
	},
	tags: ['properties-ui'],
	decorators: Story => (
		<V1BetaLayout params={{ org: 'quanta', repo: 'book' }}>
			<Story />
		</V1BetaLayout>
	),
} as Meta

const Template: StoryFn<typeof PropertiesUi> = args => (
	<PropertiesUi {...args} />
)

export const AddProperty = Template.bind({})
AddProperty.args = {
	onAddProperty: fn(),
	onUpdateProperty: fn(),
	onRemoveProperty: fn(),
	databases: [
		{
			id: '1',
			org: 'quanta',
			repo: 'book',
			items: [
				{ id: '1', name: 'Jay Gatsby' },
				{ id: '2', name: 'Nick Carraway' },
			],
		},
		{
			id: '2',
			org: 'quanta',
			repo: 'goverment',
			items: [
				{ id: '1', name: 'Atticus Finch' },
				{ id: '2', name: 'Scout Finch' },
			],
		},
		{
			id: '3',
			org: 'quanta',
			repo: 'ramen_hokkaido',
			items: [
				{ id: '1', name: 'Winston Smith' },
				{ id: '2', name: 'Julia' },
			],
		},
	],
	properties: [
		{ id: '1', name: 'id', typ: PropertyType.String },
		{ id: '2', name: 'name', typ: PropertyType.String },
		{ id: '3', name: 'createdAt', typ: PropertyType.String },
		{ id: '4', name: 'updatedAt', typ: PropertyType.String },
		{ id: '5', name: 'message', typ: PropertyType.String },
		{ id: '6', name: 'wordCount', typ: PropertyType.Integer },
		{
			id: '7',
			name: 'status',
			typ: PropertyType.Select,
			meta: {
				options: [
					{ id: '1', key: 'completed', name: 'completed' },
					{ id: '2', key: 'inReview', name: 'inReview' },
					{ id: '3', key: 'draft', name: 'draft' },
				],
			},
		},
		{ id: '8', name: 'assignedTo', typ: PropertyType.String },
		{ id: '9', name: 'location', typ: PropertyType.Location },
	],
}
AddProperty.play = async ({ canvasElement, step, args }) => {
	const canvas = within(canvasElement)

	await step('Add a new property', async () => {
		const addButton = canvas.getByText('Add New Property')
		await userEvent.click(addButton)
	})

	await step('type a new property name', async () => {
		const inputs = document.querySelectorAll('input')
		const nameInput = Array.from(inputs).find(
			i => i.placeholder === 'Enter property name',
		)
		if (!nameInput) {
			throw new Error('Name input not found')
		}
		await userEvent.type(nameInput, 'newProperty')
	})

	await step('select the property type', async () => {
		const buttons = document.querySelectorAll('button')
		const selectButton = Array.from(buttons).find(b => b.role === 'combobox')
		if (!selectButton) {
			throw new Error('Select button not found')
		}
		await userEvent.click(selectButton)
	})

	await step('select the option type', async () => {
		const options = document.querySelectorAll("[role='option']")
		const option = Array.from(options).find(o => o.textContent === 'STRING')
		if (!option) {
			throw new Error('Option not found')
		}
		await userEvent.click(option)
	})

	await step('add the property', async () => {
		const buttons = document.querySelectorAll('button')
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add Property',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)
		await waitFor(() => {
			expect(args.onAddProperty).toBeCalledWith(
				expect.objectContaining({
					name: 'newProperty',
					typ: PropertyType.String,
				}),
			)
		})
	})

	await step('verify the property was added', async () => {
		await canvas.findByText('newProperty')
	})
}

export const EditPropertyAndChangeName = Template.bind({})
EditPropertyAndChangeName.args = {
	...AddProperty.args,
	onAddProperty: fn(),
	onUpdateProperty: fn(),
	onRemoveProperty: fn(),
}
EditPropertyAndChangeName.play = async ({ canvasElement, step, args }) => {
	const canvas = within(canvasElement)

	await step('edit the property', async () => {
		const buttons = Array.from(document.querySelectorAll('button')).reverse()
		const editButton = Array.from(buttons).find(b => b.textContent === 'Edit')
		if (!editButton) {
			throw new Error('Edit button not found')
		}
		await userEvent.click(editButton)
	})

	await step('update the property name', async () => {
		const inputs = document.querySelectorAll('input')
		const nameInput = Array.from(inputs).find(
			i => i.placeholder === 'Enter property name',
		)
		if (!nameInput) {
			throw new Error('Name input not found')
		}
		await userEvent.clear(nameInput)
		await userEvent.type(nameInput, 'updatedProperty')
	})

	await step('save the property', async () => {
		const buttons = document.querySelectorAll('button')
		const saveButton = Array.from(buttons).find(
			b => b.textContent === 'Save Changes',
		)
		if (!saveButton) {
			throw new Error('Save button not found')
		}
		await userEvent.click(saveButton)
	})

	await step('verify the property was updated', async () => {
		const properties = document.querySelectorAll('tbody tr')
		const property = Array.from(properties).reverse()[0]
		expect(property.textContent).toContain('updatedProperty')
	})
}

export const RemoveProperty = Template.bind({})
RemoveProperty.args = {
	...AddProperty.args,
	onAddProperty: fn(),
	onUpdateProperty: fn(),
	onRemoveProperty: fn(),
}
RemoveProperty.play = async ({ canvasElement, step, args }) => {
	const canvas = within(canvasElement)

	await step('remove the property', async () => {
		const rows = Array.from(
			canvasElement.querySelectorAll<HTMLTableRowElement>('tbody tr'),
		)
		const targetRow = rows.find(row => row.textContent?.includes('assignedTo'))
		if (!targetRow) {
			throw new Error('Target row not found')
		}
		const removeButton = within(targetRow).getByText('Remove')
		if (!removeButton) {
			throw new Error('Remove button not found')
		}
		await userEvent.click(removeButton)
	})

	await step('verify the property was removed', async () => {
		await waitFor(() => {
			expect(args.onRemoveProperty).toBeCalledWith('8')
			expect(canvas.queryByText('assignedTo')).toBeNull()
		})
	})
}

export const AddPropertyTypeToSelect = Template.bind({})
AddPropertyTypeToSelect.args = {
	...AddProperty.args,
	onAddProperty: fn(),
	onUpdateProperty: fn(),
	onRemoveProperty: fn(),
}
AddPropertyTypeToSelect.play = async ({ canvasElement, step, args }) => {
	const canvas = within(canvasElement)

	await step('add a new property', async () => {
		const buttons = Array.from(document.querySelectorAll('button')).reverse()
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add New Property',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)
	})

	await step('type a new property name', async () => {
		const inputs = document.querySelectorAll('input')
		const nameInput = Array.from(inputs).find(
			i => i.placeholder === 'Enter property name',
		)
		if (!nameInput) {
			throw new Error('Name input not found')
		}
		await userEvent.type(nameInput, 'newProperty')
	})

	await step('select the property type', async () => {
		const buttons = document.querySelectorAll('button')
		const selectButton = Array.from(buttons).find(b => b.role === 'combobox')
		if (!selectButton) {
			throw new Error('Select button not found')
		}
		await userEvent.click(selectButton)
	})

	await step('select the select type', async () => {
		const options = document.querySelectorAll("[role='option']")
		const option = Array.from(options).find(o => o.textContent === 'SELECT')
		if (!option) {
			throw new Error('Select type not found')
		}
		await userEvent.click(option)
	})

	await step('add the option', async () => {
		const buttons = document.querySelectorAll('button')
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add Option',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)

		const inputs = document.querySelectorAll('input')
		const nameInput = Array.from(inputs).find(
			i => i.placeholder === 'Option name',
		)
		if (!nameInput) {
			throw new Error('Name input not found')
		}
		await userEvent.type(nameInput, 'newOption1')
	})

	await step('add the second option', async () => {
		const buttons = document.querySelectorAll('button')
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add Option',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)

		const inputs = document.querySelectorAll('input')
		const nameInput = Array.from(inputs)
			.reverse()
			.find(i => i.placeholder === 'Option name')
		if (!nameInput) {
			throw new Error('Name input not found')
		}
		await userEvent.type(nameInput, 'newOption2')
	})

	await step('add the property', async () => {
		const buttons = document.querySelectorAll('button')
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add Property',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)
		await waitFor(() => {
			expect(args.onAddProperty).toBeCalledWith(
				expect.objectContaining({
					name: 'newProperty',
					typ: PropertyType.Select,
					meta: expect.objectContaining({
						__typename: 'SelectType',
						options: expect.arrayContaining([
							expect.objectContaining({
								name: 'newOption1',
							}),
							expect.objectContaining({
								name: 'newOption2',
							}),
						]),
					}),
				}),
			)
		})
	})

	await step('verify the property was added', async () => {
		await canvas.findByText('newProperty')
	})
}

export const AddPropertyTypeToMultiSelect = Template.bind({})
AddPropertyTypeToMultiSelect.args = {
	...AddPropertyTypeToSelect.args,
	onAddProperty: fn(),
	onUpdateProperty: fn(),
	onRemoveProperty: fn(),
}
AddPropertyTypeToMultiSelect.play = async ({ canvasElement, step, args }) => {
	const canvas = within(canvasElement)

	await step('Add a new property', async () => {
		const addButton = canvas.getByText('Add New Property')
		await userEvent.click(addButton)
	})

	await step('type a new property name', async () => {
		const inputs = document.querySelectorAll('input')
		const nameInput = Array.from(inputs).find(
			i => i.placeholder === 'Enter property name',
		)
		if (!nameInput) {
			throw new Error('Name input not found')
		}
		await userEvent.type(nameInput, 'newMultiSelectProperty')
	})

	await step('select the property type', async () => {
		const buttons = document.querySelectorAll('button')
		const selectButton = Array.from(buttons).find(b => b.role === 'combobox')
		if (!selectButton) {
			throw new Error('Select button not found')
		}
		await userEvent.click(selectButton)
	})

	await step('select the MultiSelect type', async () => {
		const options = document.querySelectorAll("[role='option']")
		const option = Array.from(options).find(
			o => o.textContent === 'MULTI_SELECT',
		)
		if (!option) {
			throw new Error('Option not found')
		}
		await userEvent.click(option)
	})

	await step('add the first option', async () => {
		const buttons = document.querySelectorAll('button')
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add Option',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)

		const inputs = document.querySelectorAll('input')
		const optionInput = Array.from(inputs).find(
			i => i.placeholder === 'Option name',
		)
		if (!optionInput) {
			throw new Error('Option input not found')
		}
		await userEvent.type(optionInput, 'newOption1')
	})

	await step('add the second option', async () => {
		const buttons = document.querySelectorAll('button')
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add Option',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)

		const inputs = document.querySelectorAll('input')
		const optionInput = Array.from(inputs)
			.reverse()
			.find(i => i.placeholder === 'Option name')
		if (!optionInput) {
			throw new Error('Option input not found')
		}
		await userEvent.type(optionInput, 'newOption2')
	})

	await step('add the property', async () => {
		const buttons = document.querySelectorAll('button')
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add Property',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)
		await waitFor(() => {
			expect(args.onAddProperty).toBeCalledWith(
				expect.objectContaining({
					name: 'newMultiSelectProperty',
					typ: PropertyType.MultiSelect,
					meta: expect.objectContaining({
						__typename: 'MultiSelectType',
						options: expect.arrayContaining([
							expect.objectContaining({
								name: 'newOption1',
							}),
							expect.objectContaining({
								name: 'newOption2',
							}),
						]),
					}),
				}),
			)
		})
	})

	await step('verify the property was added', async () => {
		await canvas.findByText('newMultiSelectProperty')
	})
}

export const AddPropertyTypeToLocation = Template.bind({})
AddPropertyTypeToLocation.args = {
	...AddProperty.args,
	onAddProperty: fn(),
	onUpdateProperty: fn(),
	onRemoveProperty: fn(),
}
AddPropertyTypeToLocation.play = async ({ canvasElement, step, args }) => {
	const canvas = within(canvasElement)

	await step('Add a new property', async () => {
		const addButton = canvas.getByText('Add New Property')
		await userEvent.click(addButton)
	})

	await step('type a new property name', async () => {
		const inputs = document.querySelectorAll('input')
		const nameInput = Array.from(inputs).find(
			i => i.placeholder === 'Enter property name',
		)
		if (!nameInput) {
			throw new Error('Name input not found')
		}
		await userEvent.type(nameInput, 'officeLocation')
	})

	await step('select the property type', async () => {
		const buttons = document.querySelectorAll('button')
		const selectButton = Array.from(buttons).find(b => b.role === 'combobox')
		if (!selectButton) {
			throw new Error('Select button not found')
		}
		await userEvent.click(selectButton)
	})

	await step('select the Location type', async () => {
		const options = document.querySelectorAll("[role='option']")
		const option = Array.from(options).find(o => o.textContent === 'LOCATION')
		if (!option) {
			throw new Error('Location option not found')
		}
		await userEvent.click(option)
	})

	await step('add the property', async () => {
		const buttons = document.querySelectorAll('button')
		const addButton = Array.from(buttons).find(
			b => b.textContent === 'Add Property',
		)
		if (!addButton) {
			throw new Error('Add button not found')
		}
		await userEvent.click(addButton)
		await waitFor(() => {
			expect(args.onAddProperty).toBeCalledWith(
				expect.objectContaining({
					name: 'officeLocation',
					typ: PropertyType.Location,
					meta: null,
				}),
			)
		})
	})

	await step('verify the property was added', async () => {
		await canvas.findByText('officeLocation')
	})
}

// export const AddPropertyTypeToRelation = Template.bind({})
// AddPropertyTypeToRelation.args = {
// 	...AddPropertyTypeToSelect.args,
// }
// AddPropertyTypeToRelation.play = async ({ canvasElement, step, args }) => {
// 	const canvas = within(canvasElement)

// 	const previousProperties = document.querySelectorAll('tbody tr')

// 	await step('add a new property', async () => {
// 		const addButton = canvas.getByText('Add New Property')
// 		await userEvent.click(addButton)
// 	})

// 	await step('type a new property name', async () => {
// 		const inputs = document.querySelectorAll('input')
// 		const nameInput = Array.from(inputs).find(
// 			i => i.placeholder === 'Enter property name',
// 		)
// 		if (!nameInput) {
// 			throw new Error('Name input not found')
// 		}
// 		await userEvent.type(nameInput, 'newRelationProperty')
// 	})

// 	await step('select the property type', async () => {
// 		const buttons = document.querySelectorAll('button')
// 		const selectButton = Array.from(buttons).find(b => b.role === 'combobox')
// 		if (!selectButton) {
// 			throw new Error('Select button not found')
// 		}
// 		await userEvent.click(selectButton)
// 	})

// 	await step('select the relation type', async () => {
// 		const options = document.querySelectorAll("[role='option']")
// 		const option = Array.from(options).find(o => o.textContent === 'RELATION')
// 		if (!option) {
// 			throw new Error('Option not found')
// 		}
// 		await userEvent.click(option)
// 	})

// 	await step('select the database', async () => {
// 		const databaseOptions = document.querySelectorAll('#database-option')
// 		if (databaseOptions.length === 0) {
// 			throw new Error('Database option not found')
// 		}
// 		await userEvent.click(databaseOptions[0])
// 	})

// 	await step('add the property', async () => {
// 		const buttons = document.querySelectorAll('button')
// 		const addButton = Array.from(buttons).find(
// 			b => b.textContent === 'Add Property',
// 		)
// 		if (!addButton) {
// 			throw new Error('Add button not found')
// 		}
// 		await userEvent.click(addButton)
// 		await waitFor(() => {
// 			expect(args.onAddProperty).toBeCalledWith({
// 				id: '',
// 				name: 'newRelationProperty',
// 				typ: PropertyType.Relation,
// 				meta: {
// 					__typename: 'RelationType',
// 					databaseId: '1',
// 				},
// 			} satisfies PropertyForPropertiesUiFragment)
// 		})
// 	})

// 	await step('verify the property was added', async () => {
// 		const properties = document.querySelectorAll('tbody tr')
// 		expect(properties.length).toBe(previousProperties.length + 1)
// 	})
// }
