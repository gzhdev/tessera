import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { fixtures } from './fixtures'
import type { UiTree } from '../../types/generated/UiTree'

// 重绘语义与事件（任务 3.1–3.4）。

describe('重绘语义与事件', () => {
  beforeEach(() => clearWarnings())

  // 任务 3.1：id 作为比对依据——重绘前后 id 未变的输入框保持焦点。
  // 焦点保持的机制是 Vue 的 key 复用（:key="child.id"）：同一 id 的元素在重绘后
  // 仍是同一 DOM 节点，焦点自然不丢。这里用「替换兄弟节点但保留目标 id」触发局部重绘。
  it('id 未变的输入框在重绘后保持焦点', async () => {
    const treeV1: UiTree = {
      root: {
        type: 'vstack', id: 'root', props: {},
        children: [
          { type: 'text-input', id: 'endpoint', props: { value: 'https://a' } },
          { type: 'text', id: 'note', props: { text: '第一版' } },
        ],
      },
    }
    const { wrapper } = renderTree(treeV1)
    const input = wrapper.find('input.ui-text-input')
    const el = input.element as HTMLInputElement
    el.focus()
    expect(document.activeElement).toBe(el)

    // 推送新树：endpoint 的 id 与值不变，仅兄弟文本变化
    const treeV2: UiTree = {
      root: {
        type: 'vstack', id: 'root', props: {},
        children: [
          { type: 'text-input', id: 'endpoint', props: { value: 'https://a' } },
          { type: 'text', id: 'note', props: { text: '第二版' } },
        ],
      },
    }
    await wrapper.setProps({ node: treeV2.root })
    expect(wrapper.text()).toContain('第二版')
    // 同一 DOM 节点被复用，焦点仍在
    const inputAfter = wrapper.find('input.ui-text-input').element as HTMLInputElement
    expect(inputAfter).toBe(el)
    expect(document.activeElement).toBe(el)
  })

  // 任务 3.1（滚动侧）：id 未变的列表保持滚动位置（同为 key 复用的结果）。
  it('id 未变的列表在重绘后保持滚动位置', async () => {
    const items = Array.from({ length: 50 }, (_, i) => ({ title: `条目${i}` }))
    const treeV1: UiTree = {
      root: { type: 'list', id: 'lst', props: { items } },
    }
    const { wrapper } = renderTree(treeV1)
    const listEl = wrapper.find('.ui-list').element as HTMLElement
    listEl.scrollTop = 240

    const treeV2: UiTree = {
      root: { type: 'list', id: 'lst', props: { items } },
    }
    await wrapper.setProps({ node: treeV2.root })
    const listElAfter = wrapper.find('.ui-list').element as HTMLElement
    expect(listElAfter).toBe(listEl)
    expect(listElAfter.scrollTop).toBe(240)
  })

  // 任务 3.2：重复 id 触发可定位的警告（含重复值与位置）
  it('重复 id fixture 触发含重复值与位置的警告', () => {
    const { warnings } = renderTree(fixtures.duplicateId)
    const w = warnings.find((x) => x.kind === 'duplicate-id')
    expect(w).toBeDefined()
    expect(w!.message).toContain('dup')
    expect(w!.path).toContain('children[1]')
    expect(w!.message).toContain('children[0]')
  })

  // 任务 3.3：点击按钮时 handler 收到的 action 恰为节点 on 中声明的字符串
  it('点击按钮：事件打包为四字段，action 原样透传', async () => {
    const tree: UiTree = {
      root: {
        type: 'button', id: 'sync',
        props: { label: '立即同步' },
        on: { click: 'do-sync' },
      },
    }
    const { wrapper, events } = renderTree(tree)
    await wrapper.find('button').trigger('click')
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({ nodeId: 'sync', event: 'click', action: 'do-sync' })
  })

  // 任务 3.4：on 中无 click 项的按钮被点击时 handler 未被调用
  it('未声明 click 的按钮被点击时不上报', async () => {
    const tree: UiTree = {
      root: { type: 'button', id: 'silent', props: { label: '静默' } },
    }
    const { wrapper, events } = renderTree(tree)
    await wrapper.find('button').trigger('click')
    expect(events).toHaveLength(0)
  })
})
