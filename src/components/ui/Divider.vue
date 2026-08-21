<script setup lang="ts">
import type { NodeProps } from './nodeProps'

withDefaults(defineProps<NodeProps<'divider'>>(), { path: '/root' })
</script>

<template>
  <hr
    class="ui-divider"
    :class="`ui-divider--${node.props.direction ?? 'horizontal'}`"
    :aria-orientation="node.props.direction ?? 'horizontal'"
  />
</template>
