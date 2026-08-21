<script setup lang="ts">
import { computed } from 'vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'icon'>>(), { path: '/root' })

const size = computed(() => {
  const px = props.node.props.size ?? 16
  return { width: `${px}px`, height: `${px}px`, fontSize: `${px}px` }
})
</script>

<template>
  <!-- 图标以名称标识，宿主映射到具体字形；占位渲染保留语义与尺寸。 -->
  <span class="ui-icon" :data-icon="node.props.name" :style="size" role="img" :aria-label="node.props.name"></span>
</template>
