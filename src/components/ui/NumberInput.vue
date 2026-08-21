<script setup lang="ts">
// number-input：有值输入，沿用聚焦为界；上报值为数字。
import { ref, watch } from 'vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'number-input'>>(), { path: '/root' })

const localValue = ref<string>(
  props.node.props.value !== undefined ? String(props.node.props.value) : '',
)
const focused = ref(false)

watch(() => props.node.props.value, (next) => {
  if (!focused.value) localValue.value = next !== undefined ? String(next) : ''
})

function emitChange(): void {
  const action = props.node.on?.['change']
  if (action === undefined) return
  const num = localValue.value === '' ? undefined : Number(localValue.value)
  if (num === undefined || Number.isNaN(num)) return
  props.onEvent({ nodeId: props.node.id, event: 'change', action, value: num })
}

function onInput(e: Event): void {
  localValue.value = (e.target as HTMLInputElement).value
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter') emitChange()
}
</script>

<template>
  <input
    type="number"
    class="ui-number-input"
    :value="localValue"
    :min="node.props.min"
    :max="node.props.max"
    :step="node.props.step"
    @input="onInput"
    @focus="focused = true"
    @blur="focused = false; emitChange()"
    @keydown="onKeydown"
  />
</template>
