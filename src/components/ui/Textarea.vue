<script setup lang="ts">
// textarea：沿用 4.2 的本地态优先模型（任务 5.3）。
// 契约无 forceValue/debounce——只有聚焦为界 + 失焦/确认上报。
import { ref, watch } from 'vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'textarea'>>(), { path: '/root' })

const localValue = ref<string>(props.node.props.value ?? '')
const focused = ref(false)

watch(() => props.node.props.value, (next) => {
  if (!focused.value) localValue.value = next ?? ''
})

function emitChange(value: string): void {
  const action = props.node.on?.['change']
  if (action === undefined) return
  props.onEvent({ nodeId: props.node.id, event: 'change', action, value })
}

function onInput(e: Event): void {
  localValue.value = (e.target as HTMLTextAreaElement).value
}
</script>

<template>
  <textarea
    class="ui-textarea"
    :placeholder="node.props.placeholder"
    :rows="node.props.rows ?? 3"
    :value="localValue"
    @input="onInput"
    @focus="focused = true"
    @blur="focused = false; emitChange(localValue)"
  ></textarea>
</template>
