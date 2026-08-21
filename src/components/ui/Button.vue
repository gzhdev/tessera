<script setup lang="ts">
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'button'>>(), { path: '/root' })

function onClick(): void {
  // 禁用状态由描述树决定（spec「禁用组件不响应交互」）：native disabled 已挡下
  // 真实点击，这里双保险；未声明的事件不上报（任务 3.4）。
  if (props.node.props.disabled) return
  const action = props.node.on?.['click']
  if (action === undefined) return
  props.onEvent({ nodeId: props.node.id, event: 'click', action })
}
</script>

<template>
  <button
    type="button"
    class="ui-button"
    :class="`ui-button--${node.props.variant ?? 'default'}`"
    :disabled="node.props.disabled ?? false"
    @click="onClick"
  >
    {{ node.props.label }}
  </button>
</template>
