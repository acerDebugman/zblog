import { unified } from 'unified'
import remarkParse from 'remark-parse'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import remarkRehype from 'remark-rehype'
import rehypeKatex from 'rehype-katex'
import rehypeStringify from 'rehype-stringify'

/** Minimal structural subset of a hast node (avoids a new dependency). */
type HastNode = {
  type: string
  tagName?: string
  properties?: Record<string, unknown>
  children?: HastNode[]
}

const DANGEROUS_PROTOCOL = /^\s*(?:javascript|data|vbscript):/i

function stripDangerousUrls(node: HastNode): void {
  if (node.type === 'element' && node.properties) {
    const urlProps =
      node.tagName === 'a' ? ['href'] : node.tagName === 'img' ? ['src'] : []
    for (const prop of urlProps) {
      const value = node.properties[prop]
      if (typeof value === 'string' && DANGEROUS_PROTOCOL.test(value)) {
        delete node.properties[prop]
      }
    }
  }
  for (const child of node.children ?? []) stripDangerousUrls(child)
}

/** Rehype plugin: drop javascript:/data:/vbscript: URLs from links and images. */
function rehypeStripDangerousUrls() {
  return (tree: HastNode) => stripDangerousUrls(tree)
}

const pipeline = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkMath)
  .use(remarkRehype)
  .use(rehypeKatex)
  .use(rehypeStripDangerousUrls)
  .use(rehypeStringify)

/** Render Markdown source (GFM + $...$/$$...$$ math) to an HTML string. */
export async function renderMarkdown(source: string): Promise<string> {
  const file = await pipeline.process(source)
  return String(file)
}
