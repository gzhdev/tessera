<script setup lang="ts">
import type { NodeProps } from './nodeProps'

withDefaults(defineProps<NodeProps<'spinner'>>(), { path: '/root' })
</script>

<template>
  <div class="ui-spinner" role="status">
    <span class="ui-spinner__dot" aria-hidden="true"></span>
    <span v-if="node.props.label" class="ui-spinner__label">{{ node.props.label }}</span>
  </div>
</template>
