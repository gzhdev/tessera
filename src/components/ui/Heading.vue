<script setup lang="ts">
import { computed } from 'vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'heading'>>(), { path: '/root' })

// level 1–6 对应 h1–h6，越界收敛到 2。
const tag = computed(() => {
  const level = props.node.props.level ?? 2
  return `h${Math.min(6, Math.max(1, level))}`
})
</script>

<template>
  <component :is="tag" class="ui-heading">{{ node.props.text }}</component>
</template>
