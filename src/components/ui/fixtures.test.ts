import { describe, it, expect } from 'vitest'
import { fixtures } from './fixtures'
import type { UiNode } from '../../types/generated/UiNode'

// 任务 1.2：每份 fixture 都能被测试加载并通过生成的 TS 类型检查。
// 类型检查由 fixtures.ts 的 `as UiTree` + `satisfies` 在 vue-tsc 时完成；
// 这里验证运行时加载与基本形状。

function countNodes(node: UiNode): number {
  return 1 + (node.children ?? []).reduce((sum: number, c: UiNode) => sum + countNodes(c), 0)
}

describe('fixture 加载（任务 1.2）', () => {
  it('all-components 覆盖全部 34 个组件类型', () => {
    const types = new Set<string>()
    const walk = (n: UiNode) => {
      types.add(n.type)
      ;(n.children ?? []).forEach(walk)
    }
    walk(fixtures.allComponents.root)
    expect(types.size).toBe(34)
  })

  it('nested-containers 为三层嵌套', () => {
    const l1 = fixtures.nestedContainers.root
    expect(l1.type).toBe('vstack')
    expect(l1.children?.[0]?.type).toBe('hstack')
    expect(l1.children?.[0]?.children?.[0]?.type).toBe('vstack')
    expect(countNodes(l1)).toBe(5)
  })

  it('unknown-type 为 9 个已知类型节点加 1 个未知节点', () => {
    // spec「一棵含 10 个节点的树中有 1 个未知类型」：根是容器，9 个已知子节点，
    // 第 10 个（最后一个子节点）是未知类型。
    const root = fixtures.unknownType.root
    expect(root.children).toHaveLength(10)
    expect(countNodes(root)).toBe(11)
  })

  it('duplicate-id 含重复 id', () => {
    const ids: string[] = []
    const walk = (n: UiNode) => {
      ids.push(n.id)
      ;(n.children ?? []).forEach(walk)
    }
    walk(fixtures.duplicateId.root)
    expect(new Set(ids).size).toBeLessThan(ids.length)
  })
})
