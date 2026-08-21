<script setup lang="ts">
// table：结构化载荷事件代表（rowClick 携带行序号与行数据，任务 4.6）。
import type { TableRowValue } from '../../types/generated/TableRowValue'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'table'>>(), { path: '/root' })

function onRowClick(rowIndex: number, row: { [key: string]: TableRowValue }): void {
  const action = props.node.on?.['rowClick']
  if (action === undefined) return
  // value 同时携带行序号与该行数据（任务 4.6 验证点）
  props.onEvent({
    nodeId: props.node.id,
    event: 'rowClick',
    action,
    value: { rowIndex, row },
  })
}
</script>

<template>
  <table class="ui-table">
    <thead>
      <tr>
        <th v-for="col in node.props.columns" :key="col.key" :style="col.width ? { width: col.width } : undefined">
          {{ col.title }}
        </th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="(row, i) in node.props.rows" :key="i" @click="onRowClick(i, row)">
        <td v-for="col in node.props.columns" :key="col.key">{{ row[col.key] }}</td>
      </tr>
    </tbody>
  </table>
</template>
