import type { UiNode } from '../../types/generated/UiNode'
import type { UiEvent } from '../../types/generated/UiEvent'

// 所有组件共有的 props 形状：节点本体（按组件类型收窄）、定位路径、事件上报回调。
// 用法：defineProps<NodeProps<'vstack', VStackProps>>() —— node.props 即收窄为 VStackProps。

/** 把 UiNode 联合按 type 收窄后，props 字段随之收窄为对应组件的 props。 */
export type NodeOf<Tag extends UiNode['type']> = Extract<UiNode, { type: Tag }>

export interface NodeProps<Tag extends UiNode['type']> {
  node: NodeOf<Tag>
  /** 节点在树中的定位路径（重复 id / 未知类型警告用），根节点为 `/root`。 */
  path?: string
  onEvent: (e: UiEvent) => void
}
