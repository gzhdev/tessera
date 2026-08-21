<script setup lang="ts">
import type { NodeProps } from './nodeProps'

// 纯展示文本：非容器，children 不产生输出（任务 2.3）。
withDefaults(defineProps<NodeProps<'text'>>(), { path: '/root' })
</script>

<template>
  <span class="ui-text" :class="{ 'ui-text--muted': node.props.muted }">{{ node.props.text }}</span>
</template>
