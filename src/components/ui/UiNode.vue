<script setup lang="ts">
// <UiNode>：描述树节点 → 真实组件的分发器（design.md D1）。
// 按 type 从映射表取组件动态渲染；映射表未命中即未知类型，渲染占位符并记警告（D2）。
// 递归点不在此处——容器类组件在自己的模板里递归使用 <UiNode>（D1）。
import { computed } from 'vue'
import type { UiNode } from '../../types/generated/UiNode'
import type { UiEvent } from '../../types/generated/UiEvent'
import { componentMap, type ComponentTag } from './componentMap'
import { reportWarning } from './warnings'

const props = withDefaults(
  defineProps<{
    node: UiNode
    onEvent: (e: UiEvent) => void
    /** 节点在树中的定位路径（重复 id / 未知类型警告用），根节点为 `/root`。 */
    path?: string
  }>(),
  { path: '/root' },
)

// target 为映射表命中结果；命中即代表 node.type 属于该组件负责的 tag，
// 因此 node 可安全收窄为该组件的节点类型（cast 仅发生在分发这一处）。
const resolved = computed(() => {
  const tag = props.node.type as string
  const component = componentMap[tag as ComponentTag]
  if (component === undefined) {
    reportWarning({
      kind: 'unknown-component',
      path: props.path,
      message: `未识别的组件类型 \`${tag}\`，已渲染为占位符`,
    })
    return null
  }
  return { component, node: props.node as never }
})

function forward(e: UiEvent): void {
  props.onEvent(e)
}
</script>

<template>
  <component
    :is="resolved.component"
    v-if="resolved !== null"
    :node="resolved.node"
    :path="path"
    :on-event="forward"
  />
  <div v-else class="ui-unknown-placeholder" :data-type="node.type">
    无法识别的组件类型「{{ node.type }}」
  </div>
</template>
