import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { componentFixtures } from './fixtures'
import type { UiTree } from '../../types/generated/UiTree'

// code（任务 6.4）：语法高亮按需加载；未注册语言回退为转义纯文本。

function codeTree(code: string, language?: string): UiTree {
  return { root: { type: 'code', id: 'c', props: { code, language } } } as UiTree
}

describe('code 组件', () => {
  beforeEach(() => clearWarnings())

  it('已注册语言（rust）产生高亮 span', () => {
    const { wrapper } = renderTree(componentFixtures.code)
    // rust 已注册：含 hljs-keyword 等类名
    expect(wrapper.find('.hljs-keyword').exists()).toBe(true)
    expect(wrapper.text()).toContain('fn main()')
  })

  it('只加载 fixture 中实际用到的语言：未注册语言回退纯文本且转义', () => {
    // cobol 未注册 → 不高亮；内容含 HTML 敏感字符时应被转义
    const { wrapper } = renderTree(codeTree('let a = "<b>";', 'cobol'))
    expect(wrapper.find('.hljs-keyword').exists()).toBe(false)
    expect(wrapper.html()).toContain('&lt;b&gt;')
    expect(wrapper.text()).toContain('let a = "<b>";')
  })

  it('未声明语言时纯文本渲染', () => {
    const { wrapper } = renderTree(codeTree('plain text'))
    expect(wrapper.find('pre.ui-code').exists()).toBe(true)
    expect(wrapper.text()).toContain('plain text')
  })

  it('高亮输出本身已转义（代码内的 < > 不会被解释为标签）', () => {
    const { wrapper } = renderTree(codeTree('fn f<T>(x: T) {}', 'rust'))
    expect(wrapper.find('pre.ui-code').exists()).toBe(true)
    // <T> 被转义，不产生真实标签
    expect(wrapper.find('pre.ui-code t').exists()).toBe(false)
  })

  it('快照', () => {
    const { wrapper } = renderTree(componentFixtures.code)
    expect(wrapper.html()).toMatchSnapshot()
  })
})
