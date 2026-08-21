<script setup lang="ts">
// 容器子节点递归渲染点（design.md D1：递归点由容器自己决定）。
// 每个容器组件在自己的布局模板里放置 <UiChildren>，把 children 逐个交给 <UiNode>。
// 非容器组件不使用本组件，children 自然不产生输出（任务 2.3）。
import UiNodeView from './UiNode.vue'
import type { UiNode } from '../../types/generated/UiNode'
import type { UiEvent } from '../../types/generated/UiEvent'

const props = withDefaults(
  defineProps<{
    children?: UiNode[]
    /** 父节点的定位路径；子节点路径在其上派生。 */
    parentPath?: string
    onEvent: (e: UiEvent) => void
  }>(),
  { parentPath: '/root' },
)

function childPath(index: number): string {
  return `${props.parentPath}/children[${index}]`
}

function forward(e: UiEvent): void {
  props.onEvent(e)
}
</script>

<template>
  <UiNodeView
    v-for="(child, i) in children ?? []"
    :key="child.id"
    :node="child"
    :path="childPath(i)"
    :on-event="forward"
  />
</template>
