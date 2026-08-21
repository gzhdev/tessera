<script setup lang="ts">
import { computed } from 'vue'
import UiChildren from './UiChildren.vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'scroll'>>(), { path: '/root' })

const style = computed(() => ({
  overflowY: 'auto' as const,
  maxHeight: props.node.props.maxHeight !== undefined ? `${props.node.props.maxHeight}px` : undefined,
}))
</script>

<template>
  <div class="ui-scroll" :style="style">
    <UiChildren :children="node.children" :parent-path="path" :on-event="onEvent" />
  </div>
</template>
