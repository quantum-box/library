import { createReactBlockSpec } from '@blocknote/react'
import { HtmlPreviewBlockView } from './HtmlPreviewBlockView'

/**
 * A BlockNote block that holds an HTML document and renders it live in a
 * sandboxed frame — an artifact inside a rich text body.
 *
 * The source lives in `props.source`, not in inline content: the block is a
 * preview first, and inline content would invite the editor to reflow the
 * markup as text. The cost of that choice is that the Rust renderers must
 * know the type explicitly — packages/blocknote has `htmlPreview` arms in
 * to_markdown (a ```html preview fence), from_markdown, to_html and
 * plain_text. Change the shape here and those arms change with it.
 */
export const htmlPreviewBlock = createReactBlockSpec(
  {
    type: 'htmlPreview',
    propSchema: {
      source: { default: '' },
    },
    content: 'none',
  },
  {
    render: (props) => (
      <HtmlPreviewBlockView
        source={props.block.props.source}
        editable={props.editor.isEditable}
        onChange={(source) =>
          props.editor.updateBlock(props.block, { props: { source } })
        }
      />
    ),
    // What the lossy HTML/Markdown exports see. A fenced code sample is the
    // honest degraded form; the lossless path (the stored JSON and the Rust
    // renderers) keeps the real block.
    toExternalHTML: (props) => (
      <pre>
        <code className="language-html">{props.block.props.source}</code>
      </pre>
    ),
  },
)
