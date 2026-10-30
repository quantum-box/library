import { createImageBlockSpec } from '@blocknote/core'

/** Restore the stock upload button when an upload fails, without editing the document. */
export function createRecordImageBlockSpec() {
  const spec = createImageBlockSpec()
  return {
    ...spec,
    implementation: {
      ...spec.implementation,
      render(
        this: ThisParameterType<typeof spec.implementation.render>,
        ...args: Parameters<typeof spec.implementation.render>
      ) {
        const [block, editor] = args
        const view = spec.implementation.render.call(this, ...args)
        const button = view.dom.querySelector('.bn-add-file-button')
        const unsubscribe = button
          ? editor.onUploadEnd((blockId) => {
            if (blockId !== block.id || editor.getBlock(block.id)?.props.url) return
            view.dom.querySelector('.bn-file-loading-preview')?.replaceWith(button)
          })
          : undefined
        return {
          ...view,
          destroy() {
            unsubscribe?.()
            view.destroy?.()
          },
        }
      },
    },
  }
}
