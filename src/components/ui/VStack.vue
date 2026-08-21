<script setup lang="ts">
import { computed } from 'vue'
import UiChildren from './UiChildren.vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'vstack'>>(), { path: '/root' })

const style = computed(() => ({
  display: 'flex',
  flexDirection: 'column' as const,
  gap: props.node.props.gap !== undefined ? `${props.node.props.gap}px` : undefined,
  padding: props.node.props.padding !== undefined ? `${props.node.props.padding}px` : undefined,
}))
</script>

<template>
  <div class="ui-vstack" :style="style">
    <UiChildren :children="node.children" :parent-path="path" :on-event="onEvent" />
  </div>
</template>
