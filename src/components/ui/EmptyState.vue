<script setup lang="ts">
import type { NodeProps } from './nodeProps'

withDefaults(defineProps<NodeProps<'empty-state'>>(), { path: '/root' })
</script>

<template>
  <div class="ui-empty-state">
    <span v-if="node.props.icon" class="ui-empty-state__icon" :data-icon="node.props.icon"></span>
    <p class="ui-empty-state__message">{{ node.props.message }}</p>
  </div>
</template>
