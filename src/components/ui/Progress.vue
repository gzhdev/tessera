<script setup lang="ts">
import { computed } from 'vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'progress'>>(), { path: '/root' })

// value 为 0–1 的小数；缺省视为不确定进度。
const percent = computed(() => {
  const v = props.node.props.value
  if (v === undefined) return undefined
  return Math.round(Math.min(1, Math.max(0, v)) * 100)
})
</script>

<template>
  <div class="ui-progress">
    <progress
      class="ui-progress__bar"
      :value="percent"
      max="100"
    >{{ percent !== undefined ? `${percent}%` : '' }}</progress>
    <span v-if="node.props.label" class="ui-progress__label">{{ node.props.label }}</span>
  </div>
</template>
