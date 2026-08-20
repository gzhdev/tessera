# ui-component-library

## Why

`ui-schema-and-typegen` 定义了插件能描述什么，本 change 决定这些描述**长什么样、怎么响应交互**。它是 §9 那条回路（插件返回树 → 前端渲染 → 用户交互 → 事件回插件）里前端的那一半。

值得单独成一个 change 的理由是**它能完全独立验收**：拿手写的 JSON fixture 就能把全部组件、交互、降级行为跑一遍，不需要任何插件、不需要 wasmtime、不需要 Tauri IPC。这让它成为整个项目里最长的一条可并行链，应从 M0 起与后端同步推进（§18.1）。

其中有一个必须在这一层解决的硬问题：**受控输入的回环**（§9.4）。输入框每敲一个字符触发 `change` → 插件重绘 → 新树覆盖输入框 → 光标跳到末尾。这个问题只能在渲染器里解决，插件侧无能为力。

## What Changes

- `<UiNode>` 递归渲染器：按 `type` 分发到具体组件，容器类递归渲染 `children`
- §9.2 全部内置组件的 `.vue` 实现，props 严格对齐 `src/types/generated/` 的类型
- **未知类型降级**（§9.3）：渲染为错误占位符并记 warning，**不影响同一棵树的其余部分**
- **非受控输入策略**（§9.4）：文本类输入默认保留本地编辑态，仅在 `blur` 或 `Enter` 时上报 `change`；输入过程中插件推送的新树不覆盖该节点的值，除非 `props.forceValue: true`
- `props.debounce`（毫秒）：需要实时响应的场景由插件声明，前端按此节流
- **`id` 作为 diff key**（§9.3）：跨重绘保持焦点与滚动位置。`id` 的稳定性是插件作者的责任，但渲染器要在 `id` 重复时给出可诊断的 warning
- 事件打包：交互转成 §9.4 的 `{ nodeId, event, action, value }` 形状后向上抛出（本 change 只抛到一个可注入的 handler，不接真实 IPC）
- `markdown` 组件的受限子集与 HTML 转义、`code` 组件的语法高亮、`image` 组件对 `data:` URI 的支持（§9.6）

**范围边界**：不接 Tauri IPC、不接插件、不做面板容器。视图可见性、脏标记与 60ms 节流属于 `core-ui-bridge`；停靠位与 tab 属于 `ui-panel-system`。本 change 的验收全部靠 fixture 驱动。

## Capabilities

### New Capabilities

- `ui-renderer`：描述树到真实组件的渲染规则、未知类型的降级行为、`id` 的 diff 语义
- `ui-input-model`：输入类组件的受控性策略、上报时机、节流规则，以及插件重绘时本地编辑态的保留规则

### Modified Capabilities

（无）

## Impact

**新增**

- `src/components/ui/UiNode.vue`（递归分发器）与各组件 `.vue` 文件
- `src/components/ui/__fixtures__/`：覆盖全部组件、嵌套容器、未知类型、重复 id、长表格的描述树样例
- 前端测试：组件快照 + 交互行为（重点是非受控输入在重绘下的表现）

**新增依赖**

- 前端测试框架（Vitest + Vue Test Utils 或等价）
- `markdown` 与 `code` 组件所需的渲染/高亮库（选型见 design.md，需权衡包体积）

**下游影响**

- `core-ui-bridge` 把插件返回的树喂给这里的渲染器
- `ui-panel-system` 把渲染器塞进停靠面板
- `ui-settings-panel` 与 `ui-plugin-manager` 是宿主自己的界面（`components/shell/`），不走描述树，但会复用这里的基础组件样式

**风险**

- 组件数量不小（约 30 个），容易变成一次拖很久的大批量实现。建议按「先把 `<UiNode>` 与 5–6 个组件跑通，验证渲染器与 fixture 驱动的验收方式，再批量补齐」推进，而不是先写完所有组件再联调
- `markdown` 与 `code` 的库选择会显著影响前端包体积。§9.6 已经把 markdown 限定为受限子集，选型应服从这个约束而不是引入一个全功能库
