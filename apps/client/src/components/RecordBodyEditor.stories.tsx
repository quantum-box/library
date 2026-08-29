import type { Meta, StoryObj } from '@storybook/react-vite'
import { fn } from 'storybook/test'
import { RecordBodyEditor } from './RecordBodyEditor'

const artifactSource = `<!doctype html>
<html>
<head>
<style>
  body { font-family: system-ui, sans-serif; margin: 2rem; background: #f8fafc; }
  h1 { color: #1d4ed8; }
  button { padding: 0.5rem 1rem; border-radius: 0.375rem; border: 1px solid #1d4ed8; background: white; cursor: pointer; }
</style>
</head>
<body>
<h1>Sandboxed artifact</h1>
<p>Styles and scripts run inside the frame, cut off from the app.</p>
<button onclick="this.textContent = 'Clicked ' + (++this.dataset.n || (this.dataset.n = 1)) + '×'">Click me</button>
<p id="js">Scripts are blocked</p>
<script>document.getElementById('js').textContent = 'Scripts run in the sandbox ✔'</script>
</body>
</html>`

const richTextWithHtmlBlock = JSON.stringify([
  {
    id: 'b1',
    type: 'paragraph',
    content: [{ type: 'text', text: 'A rich text body with an embedded HTML artifact:', styles: {} }],
  },
  {
    id: 'b2',
    type: 'htmlPreview',
    props: { source: artifactSource },
    children: [],
  },
  {
    id: 'b3',
    type: 'paragraph',
    content: [{ type: 'text', text: 'Prose continues after the block.', styles: {} }],
  },
])

const markdownBody = [
  '# Release notes',
  '',
  'The editor keeps Markdown, HTML and RichText bodies in one component.',
  '',
  '```typescript',
  'const greet = (name: string) => `hi ${name}`',
  '```',
].join('\n')

const meta = {
  title: 'Library/RecordBodyEditor',
  component: RecordBodyEditor,
  args: {
    onCommit: fn(),
  },
  decorators: [
    (Story) => (
      <div className="max-w-[720px] p-4">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof RecordBodyEditor>

export default meta
type Story = StoryObj<typeof meta>

/** An Html Property opens as a sandboxed live preview, artifact-style. */
export const HtmlArtifact: Story = {
  args: {
    value: artifactSource,
    format: 'html',
  },
}

/** An empty Html Property opens on the Code tab, ready to write. */
export const HtmlArtifactEmpty: Story = {
  args: {
    value: '',
    format: 'html',
  },
}

/** A rich text body holding the custom htmlPreview block. */
export const RichTextWithHtmlBlock: Story = {
  args: {
    value: richTextWithHtmlBlock,
    format: 'richText',
  },
}

/** The read-only view of the same rich text body. */
export const RichTextWithHtmlBlockReadOnly: Story = {
  args: {
    value: richTextWithHtmlBlock,
    format: 'richText',
    editable: false,
  },
}

/** A Markdown body with a highlighted code block. */
export const MarkdownWithCode: Story = {
  args: {
    value: markdownBody,
  },
}

/**
 * With a repository to store them in, the image block offers an upload tab —
 * without one it can only embed an image that already has a URL.
 */
export const WithImageUploads: Story = {
  args: {
    value: markdownBody,
    surface: 'page',
    imageTarget: { org: 'acme', repo: 'handbook' },
  },
}
