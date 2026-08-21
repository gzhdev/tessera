import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { componentFixtures } from './fixtures'

// image（任务 5.2 / spec「内联图片可渲染」）。

describe('image 组件', () => {
  beforeEach(() => clearWarnings())

  it('渲染内联 data: URI 来源', () => {
    const { wrapper } = renderTree(componentFixtures.image)
    const img = wrapper.find('img.ui-image')
    expect(img.exists()).toBe(true)
    expect(img.attributes('src')).toBe('data:image/gif;base64,R0lGODlhAQABAAAAACw=')
    expect(img.attributes('alt')).toBe('占位图')
    expect(img.attributes('width')).toBe('16')
  })

  it('快照', () => {
    const { wrapper } = renderTree(componentFixtures.image)
    expect(wrapper.html()).toMatchSnapshot()
  })
})
