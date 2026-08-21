<script setup lang="ts">
import { ref } from 'vue'
import UiChildren from './UiChildren.vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'group'>>(), { path: '/root' })

// 可折叠分组的展开态是本地 UI 态；不可折叠时恒定展开。
const open = ref(true)
</script>

<template>
  <section class="ui-group">
    <header
      class="ui-group__header"
      :class="{ 'ui-group__header--collapsible': node.props.collapsible }"
      @click="node.props.collapsible ? (open = !open) : undefined"
    >
      <span v-if="node.props.collapsible" class="ui-group__caret">{{ open ? '▾' : '▸' }}</span>
      {{ node.props.title }}
    </header>
    <div v-show="open" class="ui-group__body">
      <UiChildren :children="node.children" :parent-path="path" :on-event="onEvent" />
    </div>
  </section>
</template>
