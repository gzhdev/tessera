import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { fixtures } from './fixtures'
import { componentMap, type ComponentTag } from './componentMap'
import type { UiNode } from '../../types/generated/UiNode'

// 渲染器骨架验证（任务 2.1–2.5）：分发、容器递归、非容器忽略 children、未知类型降级。

describe('渲染器骨架', () => {
  beforeEach(() => clearWarnings())

  // 任务 2.1：映射表的键与生成的 TS 联合类型对齐。
  // 类型层面由 componentMap 的 `satisfies Record<ComponentTag, Component>` 在 tsc 时保证
  // （漏加表项即编译错）；这里在运行时复核键集合与 fixture 实际覆盖一致。
  it('映射表覆盖 fixture 中出现的全部类型', () => {
    const keys = Object.keys(componentMap)
    expect(keys).toHaveLength(34)
    const seen = new Set<string>()
    const walk = (n: UiNode) => {
      seen.add(n.type)
      ;(n.children ?? []).forEach(walk)
    }
    walk(fixtures.allComponents.root)
    for (const t of seen) {
      expect(keys).toContain(t)
    }
    // 键即 ComponentTag（运行时无法直接取联合类型，但数量与覆盖已由 satisfies 保证）
    const tag: ComponentTag = 'vstack'
    expect(componentMap[tag]).toBeDefined()
  })

  // 任务 2.2：容器内部递归渲染，层级与顺序与描述树一致
  it('三层嵌套 fixture 的层级与顺序与描述树一致', () => {
    const { wrapper } = renderTree(fixtures.nestedContainers)
    const l1 = wrapper.find('.ui-vstack')
    expect(l1.exists()).toBe(true)
    const l2 = l1.find('.ui-hstack')
    expect(l2.exists()).toBe(true)
    const l3 = l2.find('.ui-vstack')
    expect(l3.exists()).toBe(true)
    const leaves = l3.findAll('.ui-text')
    expect(leaves.map((w) => w.text())).toEqual(['叶甲', '叶乙'])
  })

  // 任务 2.3：非容器组件忽略 children
  it('非容器组件携带 children 时不产生额外输出', () => {
    const tree = {
      root: {
        type: 'vstack', id: 'r', props: {},
        children: [
          { type: 'text', id: 't', props: { text: '本体' },
            children: [{ type: 'text', id: 'ghost', props: { text: '幽灵子节点' } }] } as unknown as UiNode,
        ],
      },
    }
    const { wrapper } = renderTree(tree as never)
    expect(wrapper.text()).toContain('本体')
    expect(wrapper.text()).not.toContain('幽灵子节点')
  })

  // 任务 2.4：未知类型局部降级，9 个正常渲染，第 10 个占位符，警告含类型名
  it('10 节点含 1 未知：9 个正常渲染，第 10 个占位符，警告含未识别类型名', () => {
    const { wrapper, warnings } = renderTree(fixtures.unknownType)
    const texts = wrapper.findAll('.ui-text')
    expect(texts).toHaveLength(9)
    const placeholder = wrapper.find('.ui-unknown-placeholder')
    expect(placeholder.exists()).toBe(true)
    expect(placeholder.attributes('data-type')).toBe('flying-toaster')
    const w = warnings.find((x) => x.kind === 'unknown-component')
    expect(w).toBeDefined()
    expect(w!.message).toContain('flying-toaster')
    expect(w!.path).toContain('children[9]')
  })

  // 任务 2.5：占位符内容可理解，不留空也不显示原始 JSON
  it('占位符含说明性文字，不含原始 JSON', () => {
    const { wrapper } = renderTree(fixtures.unknownType)
    const placeholder = wrapper.find('.ui-unknown-placeholder')
    expect(placeholder.text()).toContain('无法识别的组件类型')
    expect(placeholder.text()).toContain('flying-toaster')
    expect(placeholder.text()).not.toContain('{')
  })
})
