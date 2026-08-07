import { describe, expect, it } from 'vitest'
import { renderMarkdown } from './markdown'

describe('renderMarkdown', () => {
  it('renders headings and emphasis', async () => {
    const html = await renderMarkdown('# 你好 *世界*')
    expect(html).toContain('<h1>')
    expect(html).toContain('<em>世界</em>')
  })

  it('renders GFM tables', async () => {
    const html = await renderMarkdown('| a | b |\n| - | - |\n| 1 | 2 |')
    expect(html).toContain('<table>')
    expect(html).toContain('<td>2</td>')
  })

  it('renders inline math through KaTeX', async () => {
    const html = await renderMarkdown('能量公式 $E=mc^2$ 很常见')
    expect(html).toContain('class="katex"')
  })

  it('renders display math as katex-display', async () => {
    const html = await renderMarkdown('$$\n\\int_0^1 x\\,dx\n$$')
    expect(html).toContain('katex-display')
  })

  it('does not treat plain dollar amounts as math', async () => {
    const html = await renderMarkdown('价格是 5 美元，不是 $ 符号')
    expect(html).not.toContain('class="katex"')
  })
})
