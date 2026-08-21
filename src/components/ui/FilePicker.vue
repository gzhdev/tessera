<script setup lang="ts">
// file-picker：数据交付模型（设计书 §9.2 决策 B3）。
// 用户选定后只向上抛 { token, fileName, size }——永不携带任何路径字段（任务 5.5 验证点）。
// 真实 token 由宿主在接入时分配；本组件生成一次性标识并随事件交付。
import { computed } from 'vue'
import type { FilePickValue } from '../../types/generated/FilePickValue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'file-picker'>>(), { path: '/root' })

const accept = computed(() =>
  (props.node.props.accept ?? []).map((e) => `.${e.replace(/^\./, '')}`).join(','),
)

function onPick(e: Event): void {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const action = props.node.on?.['pick']
  if (action === undefined) return
  // 一次性 token：宿主侧凭它从 __picked/{token} 取内容（§11.4）。
  const token =
    typeof crypto !== 'undefined' && 'randomUUID' in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`
  // 载荷只有三字段，不含任何路径（file 的 path/name 之外的属性一律不转发）。
  const value: FilePickValue = { token, fileName: file.name, size: file.size }
  props.onEvent({ nodeId: props.node.id, event: 'pick', action, value })
  // 允许重复选择同一文件再次触发 change
  input.value = ''
}
</script>

<template>
  <label class="ui-file-picker">
    <span class="ui-file-picker__label">{{ node.props.label ?? '选择文件' }}</span>
    <input type="file" class="ui-file-picker__input" :accept="accept" @change="onPick" />
  </label>
</template>
