import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { componentFixtures } from './fixtures'

// 第一批组件快照（任务 4.1）：vstack / hstack / text / button / table。

describe('第一批组件快照（任务 4.1）', () => {
  beforeEach(() => clearWarnings())

  it('vstack', () => {
    const { wrapper } = renderTree(componentFixtures.vstack)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('hstack', () => {
    const { wrapper } = renderTree(componentFixtures.hstack)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('text（muted）', () => {
    const { wrapper } = renderTree(componentFixtures.text)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('button（primary）', () => {
    const { wrapper } = renderTree(componentFixtures.button)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('table', () => {
    const { wrapper } = renderTree(componentFixtures.table)
    expect(wrapper.html()).toMatchSnapshot()
  })

  // 5.1 布局类
  it('grid', () => {
    const { wrapper } = renderTree(componentFixtures.grid)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('scroll', () => {
    const { wrapper } = renderTree(componentFixtures.scroll)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('tabs', () => {
    const { wrapper } = renderTree(componentFixtures.tabs)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('group', () => {
    const { wrapper } = renderTree(componentFixtures.group)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('spacer', () => {
    const { wrapper } = renderTree(componentFixtures.spacer)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('split', () => {
    const { wrapper } = renderTree(componentFixtures.split)
    expect(wrapper.html()).toMatchSnapshot()
  })
  // 5.2 展示类
  it('heading', () => {
    const { wrapper } = renderTree(componentFixtures.heading)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('badge', () => {
    const { wrapper } = renderTree(componentFixtures.badge)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('divider', () => {
    const { wrapper } = renderTree(componentFixtures.divider)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('icon', () => {
    const { wrapper } = renderTree(componentFixtures.icon)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('empty-state', () => {
    const { wrapper } = renderTree(componentFixtures.emptyState)
    expect(wrapper.html()).toMatchSnapshot()
  })
  // 5.3 输入类
  it('textarea', () => {
    const { wrapper } = renderTree(componentFixtures.textarea)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('number-input', () => {
    const { wrapper } = renderTree(componentFixtures.numberInput)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('select', () => {
    const { wrapper } = renderTree(componentFixtures.select)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('checkbox', () => {
    const { wrapper } = renderTree(componentFixtures.checkbox)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('radio-group', () => {
    const { wrapper } = renderTree(componentFixtures.radioGroup)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('switch', () => {
    const { wrapper } = renderTree(componentFixtures.switch)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('slider', () => {
    const { wrapper } = renderTree(componentFixtures.slider)
    expect(wrapper.html()).toMatchSnapshot()
  })
  // 5.4 数据类与反馈类
  it('list', () => {
    const { wrapper } = renderTree(componentFixtures.list)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('tree', () => {
    const { wrapper } = renderTree(componentFixtures.tree)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('key-value', () => {
    const { wrapper } = renderTree(componentFixtures.keyValue)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('alert', () => {
    const { wrapper } = renderTree(componentFixtures.alert)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('progress', () => {
    const { wrapper } = renderTree(componentFixtures.progress)
    expect(wrapper.html()).toMatchSnapshot()
  })

  it('spinner', () => {
    const { wrapper } = renderTree(componentFixtures.spinner)
    expect(wrapper.html()).toMatchSnapshot()
  })

  // 4.1 附带验证：button 点击按 3.3 上报（属事件链路，此处复核组件自身接线）
  it('button 的点击上报 action', async () => {
    const { wrapper, events } = renderTree(componentFixtures.button)
    await wrapper.find('button').trigger('click')
    expect(events[0]).toEqual({ nodeId: 'sync', event: 'click', action: 'do-sync' })
  })
})
