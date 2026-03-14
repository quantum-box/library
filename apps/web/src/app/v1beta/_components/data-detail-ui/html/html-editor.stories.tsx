import type { Meta, StoryObj } from '@storybook/react'
import { expect, fn, userEvent, waitFor, within } from '@storybook/test'
import { RichTextEditor } from './editor'
const meta = {
	title: 'Library/DataEditor/Html/Editor',
	component: RichTextEditor,
} satisfies Meta<typeof RichTextEditor>

export default meta

type Story = StoryObj<typeof meta>

const html =
	'<div class="bn-block-group" data-node-type="blockGroup"><div class="bn-block-outer" data-node-type="blockOuter" data-id="974b8878-129b-41eb-a62d-07d482e8b76f"><div class="bn-block" data-node-type="blockContainer" data-id="974b8878-129b-41eb-a62d-07d482e8b76f"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content">Hello World</p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="26681681-4f14-4856-8744-7f71f524e773"><div class="bn-block" data-node-type="blockContainer" data-id="26681681-4f14-4856-8744-7f71f524e773"><div class="bn-block-content" data-content-type="heading" data-level="1"><h1 class="bn-inline-content">title</h1></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="f3cf53d5-3c7c-4414-8274-88af6f464b7a"><div class="bn-block" data-node-type="blockContainer" data-id="f3cf53d5-3c7c-4414-8274-88af6f464b7a"><div class="bn-block-content" data-content-type="heading" data-level="2"><h2 class="bn-inline-content">h2 title</h2></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="0db80b9f-fff3-4b1c-8651-990254e5160a"><div class="bn-block" data-node-type="blockContainer" data-id="0db80b9f-fff3-4b1c-8651-990254e5160a"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content"></p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="af657af5-ea2f-49d5-96f9-86e49394def7"><div class="bn-block" data-node-type="blockContainer" data-id="af657af5-ea2f-49d5-96f9-86e49394def7"><div class="bn-block-content" data-content-type="heading" data-level="3"><h3 class="bn-inline-content">h3 title</h3></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="2be31aeb-9fbc-4ba7-a9fd-d9b3f0c6906f"><div class="bn-block" data-node-type="blockContainer" data-id="2be31aeb-9fbc-4ba7-a9fd-d9b3f0c6906f"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content"></p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="68152e99-3f6f-496d-8136-193d240db473"><div class="bn-block" data-node-type="blockContainer" data-id="68152e99-3f6f-496d-8136-193d240db473"><div class="bn-block-content" data-content-type="bulletListItem"><p class="bn-inline-content">aaaa</p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="4fb935ed-be98-49fb-ab5d-c607cb1a8bfb"><div class="bn-block" data-node-type="blockContainer" data-id="4fb935ed-be98-49fb-ab5d-c607cb1a8bfb"><div class="bn-block-content" data-content-type="bulletListItem"><p class="bn-inline-content">aaa</p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="6955b2ea-9052-49cf-8195-a64a4e140457"><div class="bn-block" data-node-type="blockContainer" data-id="6955b2ea-9052-49cf-8195-a64a4e140457"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content"></p></div></div></div><div class="bn-block-outer" data-node-type="blockOuter" data-id="7cd9774b-e1df-45f1-b691-16e5e21bc7dd"><div class="bn-block" data-node-type="blockContainer" data-id="7cd9774b-e1df-45f1-b691-16e5e21bc7dd"><div class="bn-block-content" data-content-type="paragraph"><p class="bn-inline-content"></p></div></div></div></div>'

export const Default: Story = {
	args: {
		value: html,
		onChange: fn(),
	},
}

export const MarkdownPaste: Story = {
	args: {
		onChange: fn(),
	},
	play: async ({ canvasElement, step }) => {
		await step('Focus the editor', async () => {
			const editor = canvasElement.querySelector('[contenteditable="true"]')
			if (!editor) throw new Error('Editor not found')
			await userEvent.click(editor)
		})

		await step('Paste markdown content', async () => {
			const editor = canvasElement.querySelector('[contenteditable="true"]')
			if (!editor) throw new Error('Editor not found')
			const markdown = '# Heading 1\n\n- Item 1\n- Item 2\n\n**Bold text** here'

			const pasteEvent = new Event('paste', {
				bubbles: true,
				cancelable: true,
			})
			const dt = new DataTransfer()
			dt.setData('text/plain', markdown)
			Object.defineProperty(pasteEvent, 'clipboardData', {
				value: dt,
			})
			editor.dispatchEvent(pasteEvent)
		})

		await step('Verify heading is rendered as styled block', async () => {
			await waitFor(
				() => {
					const h1 = canvasElement.querySelector('h1')
					expect(h1).toBeTruthy()
					expect(h1?.textContent).toContain('Heading 1')
				},
				{ timeout: 3000 },
			)
		})

		await step('Verify list items are rendered', async () => {
			const canvas = within(canvasElement)
			await waitFor(() => {
				expect(canvas.getByText('Item 1')).toBeInTheDocument()
				expect(canvas.getByText('Item 2')).toBeInTheDocument()
			})
		})
	},
}

export const PlainTextPaste: Story = {
	args: {
		onChange: fn(),
	},
	play: async ({ canvasElement, step }) => {
		await step('Paste plain text', async () => {
			const editor = canvasElement.querySelector('[contenteditable="true"]')
			if (!editor) throw new Error('Editor not found')
			await userEvent.click(editor)

			const pasteEvent = new Event('paste', {
				bubbles: true,
				cancelable: true,
			})
			const dt = new DataTransfer()
			dt.setData('text/plain', 'Hello world, this is just normal text.')
			Object.defineProperty(pasteEvent, 'clipboardData', {
				value: dt,
			})
			editor.dispatchEvent(pasteEvent)
		})

		await step('Verify text is inserted as plain paragraph', async () => {
			const canvas = within(canvasElement)
			await waitFor(() => {
				expect(canvas.getByText(/Hello world/)).toBeInTheDocument()
			})
			// No styled blocks should be created for plain text
			expect(canvasElement.querySelector('h1')).toBeNull()
			expect(canvasElement.querySelector('h2')).toBeNull()
		})
	},
}
