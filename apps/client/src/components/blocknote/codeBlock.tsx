import { createCodeBlockSpec } from '@blocknote/core'
import { codeBlockOptions } from '@blocknote/code-block'
import { createRoot } from 'react-dom/client'
import { DiagramPreview } from './DiagramPreview'
import { diagramLanguage } from './diagramLanguage'

export function createDiagramCodeBlockSpec() {
  const spec = createCodeBlockSpec({ ...codeBlockOptions, defaultLanguage: 'text' })
  // The bundled highlighter has no PlantUML grammar; only the picker needs
  // those entries. Keep highlighting's supported languages unchanged.
  const pickerSpec = createCodeBlockSpec({
    ...codeBlockOptions,
    defaultLanguage: 'text',
    supportedLanguages: {
      ...codeBlockOptions.supportedLanguages,
      plantuml: { name: 'PlantUML', aliases: ['puml'] },
      puml: { name: 'PlantUML (puml)' },
    },
  })
  const render = pickerSpec.implementation.render
  return {
    ...spec,
    implementation: {
      ...spec.implementation,
      render: (function (block, editor) {
        const view = render.call(this, block, editor)
        if (this.renderType !== 'nodeView' || !diagramLanguage(block.props.language)) return view
        // Code blocks use a horizontal flex layout by default. Keep the
        // preview below the source and give large diagrams the full width.
        if (view.dom instanceof HTMLElement) view.dom.style.flexDirection = 'column'
        const preview = document.createElement('div')
        preview.style.width = '100%'
        preview.contentEditable = 'false'
        view.dom.appendChild(preview)
        const root = createRoot(preview)
        const update = (current: { props: { language: string }; content: readonly { type: string; text?: string }[] }) => {
          const language = current.props.language
          const source = current.content.map((item) => item.text ?? '').join('')
          preview.hidden = !diagramLanguage(language)
          root.render(<DiagramPreview language={language} source={source} />)
        }
        update(block)
        const unsubscribe = editor.onChange(() => {
          const current = editor.getBlock(block.id)
          if (current?.type === 'codeBlock') update(current)
        })
        return {
          ...view,
          ignoreMutation: (mutation) => mutation.type !== 'selection'
            && (mutation.target === preview || preview.contains(mutation.target)),
          destroy: () => {
            unsubscribe()
            // Node views can be destroyed during a React commit.
            queueMicrotask(() => root.unmount())
            view.destroy?.()
          },
        }
      }) satisfies typeof render,
    },
  }
}
