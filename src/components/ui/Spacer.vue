<script setup lang="ts">
import { computed } from 'vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'spacer'>>(), { path: '/root' })

const style = computed(() => {
  const size = props.node.props.size !== undefined ? `${props.node.props.size}px` : undefined
  return { width: size, height: size, flexShrink: '0' }
})
</script>

<template>
  <div class="ui-spacer" :style="style" aria-hidden="true"></div>
</template>
