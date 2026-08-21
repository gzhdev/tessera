<script setup lang="ts">
import type { NodeProps } from './nodeProps'

withDefaults(defineProps<NodeProps<'alert'>>(), { path: '/root' })
</script>

<template>
  <div class="ui-alert" :class="`ui-alert--${node.props.tone ?? 'info'}`" role="alert">
    {{ node.props.message }}
  </div>
</template>
