import { createCodeBlockSpec, type BlockImplementation } from '@blocknote/core'
import { codeBlockOptions } from '@blocknote/code-block'
import { createRoot } from 'react-dom/client'
import { DiagramPreview } from './DiagramPreview'
import { diagramLanguage } from './diagramLanguage'
import { CodeBlockStyleSelect } from './CodeBlockStyleSelect'

export function createDiagramCodeBlockSpec() {
  const spec = createCodeBlockSpec({
    ...codeBlockOptions,
    defaultLanguage: 'text',
    createHighlighter: async () => {
      const highlighter = await codeBlockOptions.createHighlighter()
      // The stock adapter selects one theme. Emit both palettes as CSS
      // variables so each block can choose its own appearance.
      const tokenize = highlighter.codeToTokens.bind(highlighter)
      highlighter.codeToTokens = (source, options) => tokenize(source, {
        ...options,
        theme: undefined,
        themes: { light: 'github-light', dark: 'github-dark' },
        defaultColor: false,
      })
      return highlighter
    },
  })
  const config = {
    ...spec.config,
    propSchema: {
      ...spec.config.propSchema,
      codeTheme: { default: 'dark', values: ['dark', 'light'] as const },
    },
  } as const
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
  const implementation: BlockImplementation<'codeBlock', typeof config.propSchema, 'inline'> = {
    ...spec.implementation,
    toExternalHTML(block, editor, context) {
      return spec.implementation.toExternalHTML?.call(
        this,
        { ...block, props: { language: block.props.language } },
        editor as unknown as Parameters<typeof render>[1],
        context,
      )
    },
    render(block, editor) {
      // Stock rendering only knows its language prop. The extra style prop
      // belongs to this schema and is carried by our wrapper's data attribute.
      const view = render.call(
        this,
        { ...block, props: { language: block.props.language } },
        editor as unknown as Parameters<typeof render>[1],
      )
      const wrapper = view.dom
      if (!(wrapper instanceof HTMLElement)) return view
      wrapper.dataset.codeTheme = block.props.codeTheme
      if (this.renderType !== 'nodeView') return view
      const styleControl = document.createElement('div')
      styleControl.className = 'library-code-style-control'
      styleControl.contentEditable = 'false'
      const styleRoot = createRoot(styleControl)
      styleRoot.render(
        <CodeBlockStyleSelect
          value={block.props.codeTheme}
          disabled={!editor.isEditable}
          onChange={(codeTheme) => editor.updateBlock(block.id, { props: { codeTheme } })}
        />,
      )
      wrapper.appendChild(styleControl)
      if (!diagramLanguage(block.props.language)) {
        return {
          ...view,
          ignoreMutation: (mutation) => styleControl.contains(mutation.target),
          destroy: () => {
            queueMicrotask(() => styleRoot.unmount())
            view.destroy?.()
          },
        }
      }
      // Code blocks use a horizontal flex layout by default. Keep the
      // preview below the source and give large diagrams the full width.
      wrapper.style.flexDirection = 'column'
      const sourceElements = Array.from(view.dom.childNodes).filter(
        (node): node is HTMLElement => node instanceof HTMLElement,
      )
      const setSourceVisibility = (visible: boolean) => {
        wrapper.classList.toggle('library-diagram-only', !visible)
        for (const element of sourceElements) {
          element.style.display = visible ? '' : 'none'
        }
      }
      const preview = document.createElement('div')
      preview.style.width = '100%'
      preview.contentEditable = 'false'
      view.dom.appendChild(preview)
      const root = createRoot(preview)
      const update = (current: { props: { language: string }; content: readonly { type: string; text?: string }[] }) => {
        const language = current.props.language
        const source = current.content.map((item) => item.text ?? '').join('')
        preview.hidden = !diagramLanguage(language)
        root.render(
          <DiagramPreview
            language={language}
            source={source}
            onSourceVisibilityChange={setSourceVisibility}
          />,
        )
      }
      update(block)
      const unsubscribe = editor.onChange(() => {
        const current = editor.getBlock(block.id)
        if (current?.type === 'codeBlock') update(current)
      })
      return {
        ...view,
        ignoreMutation: (mutation) => mutation.type !== 'selection'
          && (mutation.target === preview || preview.contains(mutation.target)
            || styleControl.contains(mutation.target)
            || (mutation.type === 'attributes' && (mutation.target === wrapper || sourceElements.includes(mutation.target as HTMLElement)))),
        destroy: () => {
          unsubscribe()
          // Node views can be destroyed during a React commit.
          queueMicrotask(() => {
            root.unmount()
            styleRoot.unmount()
          })
          view.destroy?.()
        },
      }
    },
  }
  return { ...spec, config, implementation }
}
