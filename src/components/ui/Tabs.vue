<script setup lang="ts">
import { ref, watch } from 'vue'
import UiNodeView from './UiNode.vue'
import type { UiEvent } from '../../types/generated/UiEvent'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'tabs'>>(), { path: '/root' })

// 活动页索引：本地可点击切换，新树的 active 变化时同步。
const active = ref(props.node.props.active ?? 0)
watch(() => props.node.props.active, (v) => { if (v !== undefined) active.value = v })

function forward(e: UiEvent): void { props.onEvent(e) }
</script>

<template>
  <div class="ui-tabs">
    <div class="ui-tabs__bar" role="tablist">
      <button
        v-for="(label, i) in node.props.labels"
        :key="i"
        type="button"
        role="tab"
        :aria-selected="i === active"
        class="ui-tabs__tab"
        :class="{ 'ui-tabs__tab--active': i === active }"
        @click="active = i"
      >
        {{ label }}
      </button>
    </div>
    <!-- 第 i 个页签对应第 i 个 child；只渲染活动页 -->
    <div class="ui-tabs__panel">
      <UiNodeView
        v-if="(node.children ?? [])[active]"
        :node="(node.children ?? [])[active]!"
        :path="`${path}/children[${active}]`"
        :on-event="forward"
      />
    </div>
  </div>
</template>
