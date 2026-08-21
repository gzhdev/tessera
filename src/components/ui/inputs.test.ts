import { describe, it, expect, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { renderTree, clearWarnings } from './renderTree'
import { componentFixtures } from './fixtures'
import type { UiTree } from '../../types/generated/UiTree'

// 5.3：即时型控件立即上报；textarea 沿用本地态模型；slider 按「交互进行中」处理。

describe('即时型控件立即上报（spec「即时型输入控件立即上报」）', () => {
  beforeEach(() => clearWarnings())

  it('switch 拨动立即上报新状态', async () => {
    const { wrapper, events } = renderTree(componentFixtures.switch)
    const input = wrapper.find('input[type="checkbox"]')
    ;(input.element as HTMLInputElement).checked = true
    await input.trigger('change')
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({ nodeId: 'sw', event: 'change', action: 'flip', value: true })
  })

  it('checkbox 勾选立即上报', async () => {
    const { wrapper, events } = renderTree(componentFixtures.checkbox)
    const input = wrapper.find('input[type="checkbox"]')
    ;(input.element as HTMLInputElement).checked = false
    await input.trigger('change')
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({ nodeId: 'c', event: 'change', action: 'toggle', value: false })
  })

  it('radio-group 选定立即上报', async () => {
    const { wrapper, events } = renderTree(componentFixtures.radioGroup)
    const radios = wrapper.findAll('input[type="radio"]')
    await radios[1]!.trigger('change')
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({ nodeId: 'rg', event: 'change', action: 'set-size', value: 'y' })
  })

  it('select 选定立即上报选中值', async () => {
    const { wrapper, events } = renderTree(componentFixtures.select)
    const select = wrapper.find('select')
    ;(select.element as HTMLSelectElement).value = 'b'
    await select.trigger('change')
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({ nodeId: 'sel', event: 'change', action: 'set-choice', value: 'b' })
  })

  it('slider 拖动值变化即时上报（不等失焦）', async () => {
    const { wrapper, events } = renderTree(componentFixtures.slider)
    const input = wrapper.find('input[type="range"]')
    ;(input.element as HTMLInputElement).value = '65'
    await input.trigger('input')
    expect(events.length).toBeGreaterThanOrEqual(1)
    expect(events[events.length - 1]).toEqual({ nodeId: 'sl', event: 'change', action: 'slide', value: 65 })
  })
})

describe('textarea 本地态模型（沿用 4.2）', () => {
  beforeEach(() => clearWarnings())

  it('聚焦编辑中推送新树不被覆盖；失焦上报', async () => {
    const { wrapper, events } = renderTree(componentFixtures.textarea)
    const ta = wrapper.find('textarea')
    await ta.trigger('focus')
    await ta.setValue('用户多行输入')

    const newTree: UiTree = {
      root: { type: 'textarea', id: 'ta', props: { value: '插件新值', rows: 3 }, on: { change: 'set-notes' } },
    } as UiTree
    await wrapper.setProps({ node: newTree.root })
    expect((wrapper.find('textarea').element as HTMLTextAreaElement).value).toBe('用户多行输入')

    await ta.trigger('blur')
    expect(events).toHaveLength(1)
    expect(events[0].value).toBe('用户多行输入')
  })
})

describe('slider 交互进行中不被覆盖（design.md Risks 第三行）', () => {
  beforeEach(() => clearWarnings())

  it('拖动中推送新值不覆盖；松开后新值生效', async () => {
    const { wrapper } = renderTree(componentFixtures.slider)
    const input = wrapper.find('input[type="range"]')
    // 进入拖动
    await input.trigger('pointerdown')
    ;(input.element as HTMLInputElement).value = '70'
    await input.trigger('input')

    const pushTree = (): UiTree => ({
      root: { type: 'slider', id: 'sl', props: { min: 0, max: 100, value: 10 }, on: { change: 'slide' } },
    }) as UiTree

    // 拖动中：保持本地 70
    await wrapper.setProps({ node: pushTree().root })
    expect((wrapper.find('input[type="range"]').element as HTMLInputElement).value).toBe('70')

    // 松开 → 下一次推送生效
    await input.trigger('pointerup')
    await wrapper.setProps({ node: pushTree().root })
    await nextTick()
    expect((wrapper.find('input[type="range"]').element as HTMLInputElement).value).toBe('10')
  })
})
