<script setup lang="ts">
// text-input：本地态优先模型（design.md D3）。
// 取值规则：聚焦且未声明强制取值 → 用本地值；否则用新树的值并同步本地值。
import { ref, watch } from 'vue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'text-input'>>(), { path: '/root' })

// 本地编辑态（用户输入的中间值，前端持有）
const localValue = ref<string>(props.node.props.value ?? '')
const focused = ref(false)

// 插件推来新树（node.props 引用变化）时的同步规则（D3 / spec「重绘不得覆盖正在编辑的文本值」）
watch(
  () => props.node.props,
  (next) => {
    if (props.node.props.forceValue === true) {
      // forceValue 逃生舱（任务 4.4）：即便聚焦也覆盖
      localValue.value = next.value ?? ''
      return
    }
    if (!focused.value) {
      // 未处于编辑状态：显示新值（spec「未处于编辑状态时正常更新」）
      localValue.value = next.value ?? ''
    }
    // 聚焦且未强制：保留本地值，光标不动
  },
)

// 上报：仅失焦或回车（任务 4.3）；声明了 debounce 则输入中按间隔节流上报（任务 4.5）。
let debounceTimer: ReturnType<typeof setTimeout> | null = null

function emitChange(value: string): void {
  const action = props.node.on?.['change']
  if (action === undefined) return
  props.onEvent({ nodeId: props.node.id, event: 'change', action, value })
}

function onInput(e: Event): void {
  const value = (e.target as HTMLInputElement).value
  localValue.value = value
  const interval = props.node.props.debounce
  if (interval === undefined) return
  if (debounceTimer !== null) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    debounceTimer = null
    emitChange(value)
  }, interval)
}

function onFocus(): void {
  focused.value = true
}

function onBlur(): void {
  focused.value = false
  if (debounceTimer !== null) {
    clearTimeout(debounceTimer)
    debounceTimer = null
  }
  emitChange(localValue.value)
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter') emitChange(localValue.value)
}
</script>

<template>
  <input
    type="text"
    class="ui-text-input"
    :placeholder="node.props.placeholder"
    :value="localValue"
    @input="onInput"
    @focus="onFocus"
    @blur="onBlur"
    @keydown="onKeydown"
  />
</template>
