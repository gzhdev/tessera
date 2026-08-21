<script setup lang="ts">
// slider：「交互进行中」模型（design.md Risks 第三行）——拖动期间不被新树覆盖，
// 视为广义的「编辑状态」而非严格聚焦；值变化即时上报。
import { ref, watch } from 'vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'slider'>>(), { path: '/root' })

const localValue = ref<number>(props.node.props.value ?? props.node.props.min)
const dragging = ref(false)

// 全量重绘下每次推送都是新的 node 对象，因此 watch 挂在 node 引用上：
// 拖动中（dragging）保留本地值；非拖动时同步新树的值。
watch(
  () => props.node,
  (next) => {
    if (!dragging.value && next.props.value !== undefined) localValue.value = next.props.value
  },
)

function emitChange(value: number): void {
  const action = props.node.on?.['change']
  if (action === undefined) return
  props.onEvent({ nodeId: props.node.id, event: 'change', action, value })
}

function onInput(e: Event): void {
  const value = Number((e.target as HTMLInputElement).value)
  localValue.value = value
  emitChange(value)
}
</script>

<template>
  <input
    type="range"
    class="ui-slider"
    :min="node.props.min"
    :max="node.props.max"
    :step="node.props.step"
    :value="localValue"
    @input="onInput"
    @pointerdown="dragging = true"
    @pointerup="dragging = false"
    @blur="dragging = false"
  />
</template>
