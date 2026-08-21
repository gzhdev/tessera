import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { componentFixtures } from './fixtures'
import type { FilePickValue } from '../../types/generated/FilePickValue'

// file-picker（任务 5.5 / 设计书 §9.2 决策 B3）：
// 选定后向上抛 { token, fileName, size }，载荷不含任何路径字段。

describe('file-picker 数据交付（任务 5.5）', () => {
  beforeEach(() => clearWarnings())

  it('选定后事件载荷恰为 { token, fileName, size }，无路径字段', async () => {
    const { wrapper, events } = renderTree(componentFixtures.filePicker)
    const input = wrapper.find('input[type="file"]')

    // jsdom 下构造 File 并塞进 input.files
    const file = new File(['a,b\n1,2'], 'data.csv', { type: 'text/csv' })
    Object.defineProperty(input.element, 'files', { value: [file], configurable: true })
    await input.trigger('change')

    expect(events).toHaveLength(1)
    const ev = events[0]!
    expect(ev.nodeId).toBe('fp')
    expect(ev.event).toBe('pick')
    expect(ev.action).toBe('load-csv')

    const value = ev.value as FilePickValue
    // 恰好三字段
    expect(Object.keys(value).sort()).toEqual(['fileName', 'size', 'token'])
    expect(value.fileName).toBe('data.csv')
    expect(value.size).toBe(file.size)
    expect(typeof value.token).toBe('string')
    expect(value.token.length).toBeGreaterThan(0)

    // 不含任何路径：整个事件对象序列化后无 path / webkitRelativePath 字样
    const serialized = JSON.stringify(ev)
    expect(serialized).not.toContain('path')
    expect(serialized).not.toContain('webkitRelativePath')
    expect(serialized).not.toContain('C:\\\\')
  })

  it('快照', () => {
    const { wrapper } = renderTree(componentFixtures.filePicker)
    expect(wrapper.html()).toMatchSnapshot()
  })
})
