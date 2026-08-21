// 组件类型 → Vue 组件 的映射表（design.md D1）。
// 键集合与 src/types/generated/UiNode.ts 的联合类型一一对应：
// 新增组件时漏加表项，下面的 `satisfies` 会让 tsc 报错（任务 2.1 的验收点）。
import type { Component } from 'vue'
import type { UiNode } from '../../types/generated/UiNode'

import VStackView from './VStack.vue'
import HStackView from './HStack.vue'
import TextView from './Text.vue'
import ButtonView from './Button.vue'
import TextInputView from './TextInput.vue'
import TableView from './Table.vue'
import GridView from './Grid.vue'
import ScrollView from './Scroll.vue'
import TabsView from './Tabs.vue'
import GroupView from './Group.vue'
import SpacerView from './Spacer.vue'
import SplitView from './Split.vue'
import HeadingView from './Heading.vue'
import BadgeView from './Badge.vue'
import DividerView from './Divider.vue'
import IconView from './Icon.vue'
import MarkdownView from './Markdown.vue'
import CodeView from './Code.vue'
import ImageView from './Image.vue'
import EmptyStateView from './EmptyState.vue'
import TextareaView from './Textarea.vue'
import NumberInputView from './NumberInput.vue'
import SelectView from './Select.vue'
import CheckboxView from './Checkbox.vue'
import RadioGroupView from './RadioGroup.vue'
import SwitchView from './Switch.vue'
import SliderView from './Slider.vue'
import FilePickerView from './FilePicker.vue'
import ListView from './List.vue'
import TreeView from './Tree.vue'
import KeyValueView from './KeyValue.vue'
import AlertView from './Alert.vue'
import ProgressView from './Progress.vue'
import SpinnerView from './Spinner.vue'

/** 生成类型中全部组件标记的联合（从 UiNode 判别式提取，单一事实源）。 */
export type ComponentTag = UiNode['type']

export const componentMap = {
  vstack: VStackView,
  hstack: HStackView,
  text: TextView,
  button: ButtonView,
  'text-input': TextInputView,
  table: TableView,
  grid: GridView,
  scroll: ScrollView,
  tabs: TabsView,
  group: GroupView,
  spacer: SpacerView,
  split: SplitView,
  heading: HeadingView,
  badge: BadgeView,
  divider: DividerView,
  icon: IconView,
  markdown: MarkdownView,
  code: CodeView,
  image: ImageView,
  'empty-state': EmptyStateView,
  textarea: TextareaView,
  'number-input': NumberInputView,
  select: SelectView,
  checkbox: CheckboxView,
  'radio-group': RadioGroupView,
  switch: SwitchView,
  slider: SliderView,
  'file-picker': FilePickerView,
  list: ListView,
  tree: TreeView,
  'key-value': KeyValueView,
  alert: AlertView,
  progress: ProgressView,
  spinner: SpinnerView,
} satisfies Record<ComponentTag, Component>
