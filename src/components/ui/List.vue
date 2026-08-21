<script setup lang="ts">
import type { NodeProps } from './nodeProps'

// 列表：纯展示（无交互事件）；滚动位置保持由 :key="id" 复用保证（任务 3.1）。
withDefaults(defineProps<NodeProps<'list'>>(), { path: '/root' })
</script>

<template>
  <ul class="ui-list" style="overflow-y: auto">
    <li v-for="(item, i) in node.props.items" :key="i" class="ui-list-item">
      <span class="ui-list-item__title">{{ item.title }}</span>
      <span v-if="item.subtitle" class="ui-list-item__subtitle">{{ item.subtitle }}</span>
    </li>
  </ul>
</template>
