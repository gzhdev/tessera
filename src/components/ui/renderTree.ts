import { mount, type VueWrapper } from '@vue/test-utils'
import { defineComponent, h, type PropType } from 'vue'
import type { UiTree } from '../../types/generated/UiTree'
import type { UiNode } from '../../types/generated/UiNode'
import type { UiEvent } from '../../types/generated/UiEvent'
import UiNodeView from './UiNode.vue'
import { uiWarnings, reportWarning } from './warnings'

// fixture 驱动的测试骨架（任务 1.3 / design.md D5）：
// 加载 JSON → 渲染 → 断言。事件与警告都由可注入的收集器承接，不接 IPC。

export interface RenderedTree {
  wrapper: VueWrapper
  /** 渲染期间及之后收到的全部交互事件（按到达顺序）。 */
  events: UiEvent[]
  /** 渲染期间收集到的警告（未知类型、重复 id 等），断言后由 clearWarnings 复位。 */
  warnings: typeof uiWarnings
}

// 顶层承载组件：把树的根节点交给 <UiNode>，事件 handler 以 prop 注入。
const TreeHost = defineComponent({
  props: {
    node: { type: Object as PropType<UiNode>, required: true },
    onEvent: { type: Function as PropType<(e: UiEvent) => void>, required: true },
  },
  setup(props) {
    return () => h(UiNodeView, { node: props.node, onEvent: props.onEvent })
  },
})

// 同树重复 id 检测（spec「重复 id 产生可定位的警告」）：渲染前对整树扫一遍，
// 同一 id 第二次起每次出现都记一条含重复值与位置的警告。
function checkDuplicateIds(root: UiNode): void {
  const seen = new Map<string, string>()
  const walk = (node: UiNode, path: string) => {
    const first = seen.get(node.id)
    if (first !== undefined) {
      reportWarning({
        kind: 'duplicate-id',
        path,
        message: `节点 id \`${node.id}\` 重复出现（首次位于 ${first}）`,
      })
    } else {
      seen.set(node.id, path)
    }
    ;(node.children ?? []).forEach((c, i) => walk(c, `${path}/children[${i}]`))
  }
  walk(root, '/root')
}

export interface RenderOptions {
  /** 提供给组件树的注入（如链接点击拦截 linkClickHandlerKey）。 */
  provide?: Record<string | symbol, unknown>
}

export function renderTree(tree: UiTree, options?: RenderOptions): RenderedTree {
  const events: UiEvent[] = []
  checkDuplicateIds(tree.root)
  const wrapper = mount(TreeHost, {
    // attachTo body：焦点/滚动类断言（任务 3.1）需要真实活动元素。
    attachTo: document.body,
    props: {
      node: tree.root,
      onEvent: (e: UiEvent) => events.push(e),
    },
    global: options?.provide ? { provide: options.provide } : undefined,
  })
  return { wrapper, events, warnings: uiWarnings }
}

/** 清空全局警告收集器（每个测试用例渲染前调用，避免用例间串扰）。 */
export function clearWarnings(): void {
  uiWarnings.length = 0
}
