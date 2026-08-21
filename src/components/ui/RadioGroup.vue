<script setup lang="ts">
// radio-group：即时型控件——选定即上报。
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'radio-group'>>(), { path: '/root' })

function onChange(value: string): void {
  const action = props.node.on?.['change']
  if (action === undefined) return
  props.onEvent({ nodeId: props.node.id, event: 'change', action, value })
}
</script>

<template>
  <div class="ui-radio-group" role="radiogroup">
    <label v-for="opt in node.props.options" :key="opt.value" class="ui-radio">
      <input
        type="radio"
        :name="`radio-${node.id}`"
        :value="opt.value"
        :checked="node.props.value === opt.value"
        @change="onChange(opt.value)"
      />
      <span>{{ opt.label }}</span>
    </label>
  </div>
</template>
