<script setup lang="ts">
import { ref } from 'vue'
import type { NodeProps } from './nodeProps'

withDefaults(defineProps<NodeProps<'tree'>>(), { path: '/root' })

// 树节点的展开态是本地 UI 态（不进契约）。用 Set 存已展开节点的 label 路径。
const expanded = ref<Set<string>>(new Set())
function toggle(key: string): void {
  const next = new Set(expanded.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expanded.value = next
}
function isOpen(key: string): boolean { return expanded.value.has(key) }
</script>

<template>
  <ul class="ui-tree">
    <template v-for="(item, i) in node.props.nodes" :key="i">
      <li class="ui-tree__node">
        <div
          class="ui-tree__label"
          :class="{ 'ui-tree__label--branch': (item.children ?? []).length > 0 }"
          @click="(item.children ?? []).length > 0 ? toggle(`${i}`) : undefined"
        >
          <span v-if="(item.children ?? []).length > 0" class="ui-tree__caret">
            {{ isOpen(`${i}`) ? '▾' : '▸' }}
          </span>
          {{ item.label }}
        </div>
        <ul v-if="isOpen(`${i}`)" class="ui-tree__children">
          <li v-for="(child, j) in item.children ?? []" :key="j" class="ui-tree__node">
            <div class="ui-tree__label">{{ child.label }}</div>
          </li>
        </ul>
      </li>
    </template>
  </ul>
</template>
