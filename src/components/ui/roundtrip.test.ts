import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import type { UiTree } from '../../types/generated/UiTree'

// 第一批验收（任务 4.8）：用 fixture 完整演示
// 「渲染 → 交互 → 收到事件 → 推送新树 → 输入不被破坏」的回路。
// 全程无插件、无沙箱、无 IPC——事件抛给可注入 handler（这里是数组收集器）。

function syncPanelTree(noteText: string, endpointValue: string): UiTree {
  return {
    root: {
      type: 'vstack', id: 'panel', props: { gap: 8 },
      children: [
        { type: 'text', id: 'title', props: { text: '同步面板' } },
        { type: 'text-input', id: 'endpoint', props: { value: endpointValue }, on: { change: 'set-endpoint' } },
        { type: 'button', id: 'sync', props: { label: '同步', variant: 'primary' }, on: { click: 'do-sync' } },
        { type: 'text', id: 'note', props: { text: noteText, muted: true } },
      ],
    },
  } as UiTree
}

describe('第一批验收：完整回路（任务 4.8）', () => {
  beforeEach(() => clearWarnings())

  it('渲染 → 交互 → 事件 → 推送新树 → 输入不被破坏', async () => {
    // 1. 渲染初始树
    const { wrapper, events } = renderTree(syncPanelTree('等待同步', 'https://a'))
    expect(wrapper.text()).toContain('同步面板')
    expect(wrapper.text()).toContain('等待同步')

    // 2. 用户开始编辑 endpoint（聚焦 + 键入）
    const input = wrapper.find('input.ui-text-input')
    const el = input.element as HTMLInputElement
    el.focus()
    await input.trigger('focus')
    await input.setValue('https://用户编辑中')

    // 3. 用户点击「同步」按钮 → 收到事件
    await wrapper.find('button.ui-button').trigger('click')
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({ nodeId: 'sync', event: 'click', action: 'do-sync' })

    // 4. 插件响应后推送新树：状态文本变化、endpoint 被插件改写
    await wrapper.setProps({ node: syncPanelTree('同步完成', 'https://插件改写').root })
    expect(wrapper.text()).toContain('同步完成')

    // 5. endpoint 聚焦编辑中 → 用户内容不被新树覆盖，光标不动
    const elAfter = wrapper.find('input.ui-text-input').element as HTMLInputElement
    expect(elAfter.value).toBe('https://用户编辑中')
    expect(document.activeElement).toBe(elAfter)

    // 6. 失焦 → 上报用户最终内容
    await wrapper.find('input.ui-text-input').trigger('blur')
    expect(events).toHaveLength(2)
    expect(events[1]).toEqual({
      nodeId: 'endpoint', event: 'change', action: 'set-endpoint', value: 'https://用户编辑中',
    })
  })
})
