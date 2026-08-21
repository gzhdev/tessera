import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import type { UiTree } from '../../types/generated/UiTree'

// 输入模型（任务 4.2–4.5）：本地态优先、上报时机、forceValue、debounce。
// 以及 4.7 禁用状态。

function inputTree(props: Record<string, unknown>, on?: Record<string, string>): UiTree {
  return {
    root: { type: 'text-input', id: 'endpoint', props, on },
  } as UiTree
}

describe('text-input 输入模型', () => {
  beforeEach(() => {
    clearWarnings()
    vi.useRealTimers()
  })
  afterEach(() => vi.useRealTimers())

  // 4.2：输入过程中推送含不同值的新树，输入框保留用户内容且光标不动
  it('聚焦编辑中推送新树：保留用户内容，光标不动', async () => {
    const { wrapper } = renderTree(inputTree({ value: '初始' }))
    const input = wrapper.find('input')
    const el = input.element as HTMLInputElement
    el.focus()
    await input.trigger('focus')
    await input.setValue('用户正在输入')
    el.setSelectionRange(5, 5)

    // 推送新树：值不同，但节点聚焦中 → 本地值不被覆盖
    await wrapper.setProps({ node: inputTree({ value: '插件新值' }).root })
    expect((wrapper.find('input').element as HTMLInputElement).value).toBe('用户正在输入')
    expect(el.selectionStart).toBe(5)
  })

  // 4.2：未聚焦时推送新值则显示新值
  it('未聚焦时推送新树：显示新值', async () => {
    const { wrapper } = renderTree(inputTree({ value: '初始' }))
    expect((wrapper.find('input').element as HTMLInputElement).value).toBe('初始')
    await wrapper.setProps({ node: inputTree({ value: '更新后' }).root })
    expect((wrapper.find('input').element as HTMLInputElement).value).toBe('更新后')
  })

  // 4.3：键入 10 个字符期间零上报
  it('键入期间零上报；失焦与回车各上报一次完整内容', async () => {
    const { wrapper, events } = renderTree(inputTree({ value: '' }, { change: 'set-endpoint' }))
    const input = wrapper.find('input')
    await input.trigger('focus')
    // 逐字符键入 10 个字符
    for (let i = 1; i <= 10; i++) {
      const v = 'x'.repeat(i)
      ;(input.element as HTMLInputElement).value = v
      await input.trigger('input')
    }
    expect(events).toHaveLength(0)

    // 回车上报一次
    await input.trigger('keydown', { key: 'Enter' })
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({
      nodeId: 'endpoint', event: 'change', action: 'set-endpoint', value: 'xxxxxxxxxx',
    })

    // 再失焦上报一次
    await input.trigger('blur')
    expect(events).toHaveLength(2)
    expect(events[1].value).toBe('xxxxxxxxxx')
  })

  // 4.4：forceValue 逃生舱——聚焦中也被替换
  it('新树声明 forceValue 时，聚焦中内容也被替换', async () => {
    const { wrapper } = renderTree(inputTree({ value: '初始' }))
    const input = wrapper.find('input')
    await input.trigger('focus')
    await input.setValue('用户输入')

    await wrapper.setProps({ node: inputTree({ value: '强制覆盖', forceValue: true }).root })
    expect((wrapper.find('input').element as HTMLInputElement).value).toBe('强制覆盖')
  })

  // 4.5：声明 300ms 后连续键入 2 秒，上报次数有上限；未声明时输入中零上报
  it('debounce 300ms：连续键入 2 秒上报次数受限；未声明则输入中零上报', async () => {
    vi.useFakeTimers()
    const { wrapper, events } = renderTree(inputTree({ value: '', debounce: 300 }, { change: 'set-endpoint' }))
    const input = wrapper.find('input')
    await input.trigger('focus')

    // 连续键入 2 秒：每 50ms 一次 input（模拟持续键入）
    for (let t = 0; t < 2000; t += 50) {
      ;(input.element as HTMLInputElement).value = `v${t}`
      await input.trigger('input')
      vi.advanceTimersByTime(50)
    }
    // trailing 节流：2 秒内至多最后一次落定后触发 1 次（远低于逐字符的 40 次）
    vi.advanceTimersByTime(300)
    expect(events.length).toBeLessThanOrEqual(1)
    expect(events.length).toBeGreaterThanOrEqual(1)
    vi.useRealTimers()

    // 未声明 debounce：输入中零上报
    const r2 = renderTree(inputTree({ value: '' }, { change: 'set-endpoint' }))
    const input2 = r2.wrapper.find('input')
    await input2.trigger('focus')
    for (let i = 0; i < 5; i++) {
      ;(input2.element as HTMLInputElement).value = `y${i}`
      await input2.trigger('input')
    }
    expect(r2.events).toHaveLength(0)
  })
})

describe('禁用状态（任务 4.7）', () => {
  beforeEach(() => clearWarnings())

  it('禁用的按钮点击不产生上报；改为可用后重新响应', async () => {
    const disabledTree: UiTree = {
      root: { type: 'button', id: 'b', props: { label: '禁用', disabled: true }, on: { click: 'go' } },
    }
    const { wrapper, events } = renderTree(disabledTree)
    await wrapper.find('button').trigger('click')
    expect(events).toHaveLength(0)

    // 新树把禁用改为可用
    const enabledTree: UiTree = {
      root: { type: 'button', id: 'b', props: { label: '可用', disabled: false }, on: { click: 'go' } },
    }
    await wrapper.setProps({ node: enabledTree.root })
    await wrapper.find('button').trigger('click')
    expect(events).toHaveLength(1)
    expect(events[0].action).toBe('go')
  })
})
