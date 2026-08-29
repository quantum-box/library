/**
 * Markdown cannot say how wide an image was resized to: `![alt](url)` has
 * no attribute slot, and BlockNote's importer drops raw `<img>` HTML
 * entirely. The width therefore rides in a URL fragment — `#w=256` — which
 * never reaches the server, renders as a plain image anywhere else the
 * Markdown goes, and survives the round trip that used to reset every
 * resize on reload. `withImageWidthFragments` encodes before a Markdown
 * export and `takeImageWidthFragments` decodes after an import; both leave
 * every other block untouched. The Rust renderers speak the same dialect
 * (packages/blocknote), so API-produced Markdown round-trips too.
 */

const IMAGE_WIDTH_FRAGMENT = /#w=(\d+)$/

interface BlockShape {
  type?: unknown
  props?: unknown
  children?: unknown
}

type ImageProps = Record<string, unknown>

export function withImageWidthFragments<T extends BlockShape>(blocks: T[]): T[] {
  return mapImageBlocks(blocks, (props) => {
    const width = props.previewWidth
    const url = props.url
    if (typeof width !== 'number' || !Number.isFinite(width) || width <= 0) return props
    if (typeof url !== 'string' || url === '' || IMAGE_WIDTH_FRAGMENT.test(url)) return props
    return { ...props, url: `${url}#w=${Math.round(width)}` }
  })
}

export function takeImageWidthFragments<T extends BlockShape>(blocks: T[]): T[] {
  return mapImageBlocks(blocks, (props) => {
    const url = props.url
    if (typeof url !== 'string') return props
    const match = IMAGE_WIDTH_FRAGMENT.exec(url)
    if (!match) return props
    return {
      ...props,
      url: url.slice(0, -match[0].length),
      previewWidth: Number(match[1]),
    }
  })
}

function mapImageBlocks<T extends BlockShape>(
  blocks: T[],
  mapProps: (props: ImageProps) => ImageProps,
): T[] {
  return blocks.map((block) => {
    let next = block
    if (block.type === 'image' && block.props && typeof block.props === 'object') {
      const props = mapProps(block.props as ImageProps)
      if (props !== block.props) next = { ...block, props }
    }
    if (Array.isArray(block.children) && block.children.length > 0) {
      next = { ...next, children: mapImageBlocks(block.children as BlockShape[], mapProps) }
    }
    return next
  })
}
