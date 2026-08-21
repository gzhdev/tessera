<script setup lang="ts">
// checkbox：即时型控件——切换即上报（spec「即时型输入控件立即上报」）。
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'checkbox'>>(), { path: '/root' })

function onChange(e: Event): void {
  const checked = (e.target as HTMLInputElement).checked
  const action = props.node.on?.['change']
  if (action === undefined) return
  props.onEvent({ nodeId: props.node.id, event: 'change', action, value: checked })
}
</script>

<template>
  <label class="ui-checkbox">
    <input type="checkbox" :checked="node.props.checked ?? false" @change="onChange" />
    <span v-if="node.props.label">{{ node.props.label }}</span>
  </label>
</template>
