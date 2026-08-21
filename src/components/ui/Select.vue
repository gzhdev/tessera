<script setup lang="ts">
// select：即时型控件——选定即上报，不等失焦（spec「下拉选择立即上报」）。
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'select'>>(), { path: '/root' })

function onChange(e: Event): void {
  const value = (e.target as HTMLSelectElement).value
  const action = props.node.on?.['change']
  if (action === undefined) return
  props.onEvent({ nodeId: props.node.id, event: 'change', action, value })
}
</script>

<template>
  <select class="ui-select" :value="node.props.value" @change="onChange">
    <option v-if="node.props.placeholder" value="" disabled>{{ node.props.placeholder }}</option>
    <option v-for="opt in node.props.options" :key="opt.value" :value="opt.value">
      {{ opt.label }}
    </option>
  </select>
</template>
