<script setup lang="ts">
import { computed } from 'vue'
import UiNodeView from './UiNode.vue'
import type { UiEvent } from '../../types/generated/UiEvent'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'split'>>(), { path: '/root' })

// 两栏容器：children[0] 与 children[1] 按 ratio 分配；direction 决定主轴。
const style = computed(() => ({
  display: 'flex',
  flexDirection: (props.node.props.direction === 'vertical' ? 'column' : 'row') as 'column' | 'row',
}))

function paneStyle(index: number) {
  const ratio = props.node.props.ratio ?? 0.5
  const fraction = index === 0 ? ratio : 1 - ratio
  return { flexGrow: String(fraction), flexBasis: '0' }
}

function forward(e: UiEvent): void { props.onEvent(e) }
</script>

<template>
  <div class="ui-split" :class="`ui-split--${node.props.direction ?? 'horizontal'}`" :style="style">
    <div
      v-for="(child, i) in (node.children ?? []).slice(0, 2)"
      :key="child.id"
      class="ui-split__pane"
      :style="paneStyle(i)"
    >
      <UiNodeView :node="child" :path="`${path}/children[${i}]`" :on-event="forward" />
    </div>
  </div>
</template>
