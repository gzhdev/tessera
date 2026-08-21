<script setup lang="ts">
import type { NodeProps } from './nodeProps'

withDefaults(defineProps<NodeProps<'key-value'>>(), { path: '/root' })
</script>

<template>
  <dl class="ui-key-value">
    <template v-for="(entry, i) in node.props.entries" :key="i">
      <dt class="ui-key-value__key">{{ entry.key }}</dt>
      <dd class="ui-key-value__value">{{ entry.value }}</dd>
    </template>
  </dl>
</template>
