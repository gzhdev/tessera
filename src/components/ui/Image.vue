<script setup lang="ts">
import type { NodeProps } from './nodeProps'

// 图片来源接受 `data:` URI 与虚拟路径（设计书 §9.6），原样作为 src。
withDefaults(defineProps<NodeProps<'image'>>(), { path: '/root' })
</script>

<template>
  <img
    class="ui-image"
    :src="node.props.source"
    :alt="node.props.alt ?? ''"
    :width="node.props.width"
  />
</template>
