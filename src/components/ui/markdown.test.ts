import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { componentFixtures } from './fixtures'
import { linkClickHandlerKey } from './linkClick'
import type { UiTree } from '../../types/generated/UiTree'

// markdown（任务 6.2 / 6.3）：受限子集渲染、HTML 转义、链接点击拦截。

function mdTree(source: string): UiTree {
  return { root: { type: 'markdown', id: 'md', props: { source } } } as UiTree
}

describe('markdown 组件', () => {
  beforeEach(() => clearWarnings())

  // 6.2：受限子集渲染（标题、列表、强调、链接、表格、代码块）
  it('渲染受限子集元素', () => {
    const { wrapper } = renderTree(
      mdTree('# T\n\n- 甲\n- 乙\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n`code` **强调** [x](https://a.com)'),
    )
    expect(wrapper.find('h1').text()).toBe('T')
    expect(wrapper.findAll('li')).toHaveLength(2)
    expect(wrapper.find('table').exists()).toBe(true)
    expect(wrapper.find('code').text()).toBe('code')
    expect(wrapper.find('strong').text()).toBe('强调')
    expect(wrapper.find('a[data-md-link]').exists()).toBe(true)
  })

  // 6.2：含 <script> 的内容以字面文本显示，不被解释为标记
  it('HTML 注入被转义为字面文本', () => {
    const { wrapper } = renderTree(mdTree('前文\n\n<script>alert(1)</script>\n\n后文'))
    // 没有真的 script 元素
    expect(wrapper.find('script').exists()).toBe(false)
    // 以字面文本呈现
    expect(wrapper.text()).toContain('<script>alert(1)</script>')
    // 原始 HTML 中是被转义的实体
    expect(wrapper.html()).toContain('&lt;script&gt;')
  })

  // 6.2：图片不在受限子集，降级为字面文本（图片走 image 组件）
  it('图片语法降级为字面文本', () => {
    const { wrapper } = renderTree(mdTree('![图](x.png) 后文'))
    expect(wrapper.find('img').exists()).toBe(false)
    expect(wrapper.text()).toContain('![图](x.png)')
  })

  // 6.3：点击链接不跳转，触发拦截回调
  it('链接点击被拦截：页面不跳转，触发宿主回调', async () => {
    const intercepted: string[] = []
    const { wrapper } = renderTree(mdTree('[外部](https://example.com)'), {
      provide: { [linkClickHandlerKey as symbol]: (href: string) => intercepted.push(href) },
    })
    const anchor = wrapper.find('a[data-md-link]')
    expect(anchor.exists()).toBe(true)
    await anchor.trigger('click')
    expect(intercepted).toEqual(['https://example.com'])
  })

  it('快照', () => {
    const { wrapper } = renderTree(componentFixtures.markdown)
    expect(wrapper.html()).toMatchSnapshot()
  })
})
