<script setup lang="ts">
import { computed } from 'vue'
import UiChildren from './UiChildren.vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'grid'>>(), { path: '/root' })

const style = computed(() => ({
  display: 'grid',
  gridTemplateColumns:
    props.node.props.columns !== undefined ? `repeat(${props.node.props.columns}, 1fr)` : undefined,
  gap: props.node.props.gap !== undefined ? `${props.node.props.gap}px` : undefined,
}))
</script>

<template>
  <div class="ui-grid" :style="style">
    <UiChildren :children="node.children" :parent-path="path" :on-event="onEvent" />
  </div>
</template>
