<script setup lang="ts">
import type { NodeProps } from './nodeProps'

withDefaults(defineProps<NodeProps<'badge'>>(), { path: '/root' })
</script>

<template>
  <span class="ui-badge" :class="`ui-badge--${node.props.tone ?? 'info'}`">{{ node.props.text }}</span>
</template>
