import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { fixtures } from './fixtures'

// fixture 驱动的测试骨架（任务 1.3）：加载 JSON → 渲染 → 断言。
// 骨架本身已可跑通；组件实现由 2.x/4.x/5.x 任务逐步替换。

describe('fixture 驱动测试骨架（任务 1.3）', () => {
  beforeEach(() => clearWarnings())

  it('加载 nested-containers：三层容器嵌套被展开，叶子文本可见', () => {
    const { wrapper } = renderTree(fixtures.nestedContainers)
    expect(wrapper.find('.ui-vstack').exists()).toBe(true)
    expect(wrapper.find('.ui-hstack').exists()).toBe(true)
    // 叶子 text 经三层递归渲染出来
    expect(wrapper.text()).toContain('叶甲')
    expect(wrapper.text()).toContain('叶乙')
  })

  it('加载 unknown-type：未知类型走占位符路径', () => {
    const { wrapper } = renderTree(fixtures.unknownType)
    expect(wrapper.find('.ui-unknown-placeholder').exists()).toBe(true)
  })
})
