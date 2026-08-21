<script setup lang="ts">
// switch：即时型控件——拨动即上报（spec「开关立即上报」）。
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'switch'>>(), { path: '/root' })

function onChange(e: Event): void {
  const on = (e.target as HTMLInputElement).checked
  const action = props.node.on?.['change']
  if (action === undefined) return
  props.onEvent({ nodeId: props.node.id, event: 'change', action, value: on })
}
</script>

<template>
  <label class="ui-switch">
    <input type="checkbox" role="switch" :checked="node.props.on ?? false" @change="onChange" />
    <span v-if="node.props.label">{{ node.props.label }}</span>
  </label>
</template>
