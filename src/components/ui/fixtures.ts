import type { UiTree } from '../../types/generated/UiTree'

import allComponents from './__fixtures__/all-components.json'
import nestedContainers from './__fixtures__/nested-containers.json'
import unknownType from './__fixtures__/unknown-type.json'
import duplicateId from './__fixtures__/duplicate-id.json'
import bigTable from './__fixtures__/big-table.json'
import markdownInjection from './__fixtures__/markdown-injection.json'

import compVStack from './__fixtures__/components/vstack.json'
import compHStack from './__fixtures__/components/hstack.json'
import compText from './__fixtures__/components/text.json'
import compButton from './__fixtures__/components/button.json'
import compTable from './__fixtures__/components/table.json'
import compGrid from './__fixtures__/components/grid.json'
import compScroll from './__fixtures__/components/scroll.json'
import compTabs from './__fixtures__/components/tabs.json'
import compGroup from './__fixtures__/components/group.json'
import compSpacer from './__fixtures__/components/spacer.json'
import compSplit from './__fixtures__/components/split.json'
import compHeading from './__fixtures__/components/heading.json'
import compBadge from './__fixtures__/components/badge.json'
import compDivider from './__fixtures__/components/divider.json'
import compIcon from './__fixtures__/components/icon.json'
import compImage from './__fixtures__/components/image.json'
import compEmptyState from './__fixtures__/components/empty-state.json'
import compTextarea from './__fixtures__/components/textarea.json'
import compNumberInput from './__fixtures__/components/number-input.json'
import compSelect from './__fixtures__/components/select.json'
import compCheckbox from './__fixtures__/components/checkbox.json'
import compRadioGroup from './__fixtures__/components/radio-group.json'
import compSwitch from './__fixtures__/components/switch.json'
import compSlider from './__fixtures__/components/slider.json'
import compList from './__fixtures__/components/list.json'
import compTree from './__fixtures__/components/tree.json'
import compKeyValue from './__fixtures__/components/key-value.json'
import compAlert from './__fixtures__/components/alert.json'
import compProgress from './__fixtures__/components/progress.json'
import compSpinner from './__fixtures__/components/spinner.json'
import compFilePicker from './__fixtures__/components/file-picker.json'
import compMarkdown from './__fixtures__/components/markdown.json'
import compCode from './__fixtures__/components/code.json'

// vite 的 json 导入产物是任意对象，这里显式收窄为 UiTree：
// fixture 若有形状错误，vue-tsc 在这几行就会报错（任务 1.2 的验收点）。
export const fixtures = {
  allComponents: allComponents as UiTree,
  nestedContainers: nestedContainers as UiTree,
  unknownType: unknownType as UiTree,
  duplicateId: duplicateId as UiTree,
  bigTable: bigTable as UiTree,
  markdownInjection: markdownInjection as UiTree,
} satisfies Record<string, UiTree>

/** 单组件 fixture（快照测试用）。 */
export const componentFixtures = {
  vstack: compVStack as UiTree,
  hstack: compHStack as UiTree,
  text: compText as UiTree,
  button: compButton as UiTree,
  table: compTable as UiTree,
  grid: compGrid as UiTree,
  scroll: compScroll as UiTree,
  tabs: compTabs as UiTree,
  group: compGroup as UiTree,
  spacer: compSpacer as UiTree,
  split: compSplit as UiTree,
  heading: compHeading as UiTree,
  badge: compBadge as UiTree,
  divider: compDivider as UiTree,
  icon: compIcon as UiTree,
  image: compImage as UiTree,
  emptyState: compEmptyState as UiTree,
  textarea: compTextarea as UiTree,
  numberInput: compNumberInput as UiTree,
  select: compSelect as UiTree,
  checkbox: compCheckbox as UiTree,
  radioGroup: compRadioGroup as UiTree,
  switch: compSwitch as UiTree,
  slider: compSlider as UiTree,
  list: compList as UiTree,
  tree: compTree as UiTree,
  keyValue: compKeyValue as UiTree,
  alert: compAlert as UiTree,
  progress: compProgress as UiTree,
  spinner: compSpinner as UiTree,
  filePicker: compFilePicker as UiTree,
  markdown: compMarkdown as UiTree,
  code: compCode as UiTree,
} satisfies Record<string, UiTree>
