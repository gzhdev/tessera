import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { componentFixtures } from './fixtures'

// 任务 4.6：table 行点击事件的 value 同时携带行序号与该行数据。

describe('table 行点击（任务 4.6）', () => {
  beforeEach(() => clearWarnings())

  it('行点击时 value 携带 rowIndex 与 row', async () => {
    const { wrapper, events } = renderTree(componentFixtures.table)
    const rows = wrapper.findAll('tbody tr')
    expect(rows).toHaveLength(2)
    await rows[1]!.trigger('click')
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({
      nodeId: 'files',
      event: 'rowClick',
      action: 'open-file',
      value: { rowIndex: 1, row: { name: 'b.txt', status: '待处理' } },
    })
  })

  it('未声明 rowClick 时点击不上报', async () => {
    const tree = {
      root: {
        type: 'table', id: 't',
        props: { columns: [{ key: 'a', title: 'A' }], rows: [{ a: 1 }] },
      },
    }
    const { wrapper, events } = renderTree(tree as never)
    await wrapper.find('tbody tr').trigger('click')
    expect(events).toHaveLength(0)
  })
})
