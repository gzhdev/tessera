# ui-schema-and-typegen

## Why

§9 决定了插件不碰 DOM，只返回一棵 JSON 描述树，由宿主前端渲染。这棵树的形状就是**插件与宿主之间的第二条契约**（第一条是 WIT）。它同时被三方消费：插件 SDK 的 `ui::` builder 生成它、宿主校验它、前端渲染它。

三方共用一份契约，就必须有单一真值。§17.2 已经定了做法——「`types/generated/` 由构建脚本从 Rust 类型生成，**不手写、不手改**，纳入 git 以便 review 契约变更」。v0.3 把这条提升为 §17.1 的硬性约束 5。本 change 就是把这条约束在 UI 契约上落地。

排期上这块值得强调：**它对 wasmtime 零依赖，能靠手写 JSON fixture 独立验收**。设计书 §18.1 把 UI 线标为「唯一的长并行链」——若拖到 M3 才启动，它会成为 M3 的关键路径。应从 M0 就与后端并行开工。

## What Changes

- §9.2 组件清单的 Rust 类型定义：布局（`vstack` `hstack` `grid` `scroll` `tabs` `group` `spacer` `split`）、展示（`text` `heading` `badge` `divider` `icon` `markdown` `code` `image` `empty-state`）、输入（`button` `text-input` `textarea` `number-input` `select` `checkbox` `radio-group` `switch` `slider` `file-picker`）、数据（`table` `list` `tree` `key-value`）、反馈（`alert` `progress` `spinner`）
- §9.3 节点 schema：`type` / `id` / `props` / `children` / `on` 五个字段的类型与约束
- §9.4 UI 事件的类型：`nodeId` / `event` / `action` / `value`，含 `file-picker` 的 `{ token, fileName, size }` 形态（v0.3 决策 B3）
- **TS 类型生成管线**：`ts-rs` 或等价工具从 Rust 类型产出 `src/types/generated/`，纳入 git，CI 校验「重新生成后与 check-in 内容逐字节一致」
- 描述树的校验与降级：§9.3 规定未知组件类型**渲染为错误占位符并记 warning，不影响其余部分**——校验器给出结构化的降级结果而非整树拒绝
- `text-input` **不提供** `password` prop（v0.3 细则）：凭据统一走配置项，宿主无法在插件的 UI 树里强制风险说明

**范围边界**：本 change 只定义契约与生成管线，**不实现任何 `.vue` 组件**（见 `ui-component-library`），也不接任何插件（见 `core-ui-bridge`）。

## Capabilities

### New Capabilities

- `ui-schema`：UI 描述树的节点结构、组件类型清单与各自的 props 契约、UI 事件的载荷形状，以及未知类型的降级规则
- `ui-typegen`：Rust 类型到 TypeScript 类型的生成管线，及其一致性校验规则

### Modified Capabilities

（无）

## Impact

**新增**

- `crates/tessera-ui-schema/src/{node.rs, components.rs, event.rs, tree.rs, validate.rs, bin/gen-types.rs}`（实现为按职责分文件的单层模块，`components.rs` 收拢 34 个 props 契约）
- `src/types/generated/`（**生成物**，纳入 git）
- 生成脚本与 CI 一致性检查
- `crates/tessera-ui-schema/tests/fixtures/`：正例树、未知类型树、非法 props 树

**新增依赖**

- `ts-rs`（或等价工具，见 design.md 的取舍）、`serde`、`serde_json`（校验入口的输入形态）

**下游影响**

- `ui-component-library` 按这里的类型实现 `.vue` 组件，两者永远一起动
- `core-plugin-sdk` 的 `ui::` builder 在编译期保证生成的树符合这里的类型
- `core-ui-bridge` 在把插件返回的 JSON 推给前端之前用这里的校验器过一遍

**风险**

- `ts-rs` 对复杂 Rust 类型（尤其是带泛型或自定义 serde 属性的 enum）的生成质量需要早验证。若产出的 TS 类型不可用，退路是 `schemars` + `json-schema-to-typescript`，但那会引入两套生成管线
- 组件清单一旦发布就受 §13.2 的 `CORE_API_LEVEL` 约束（删组件或改 props 语义要 +1）。MVP 阶段应把握「宁可少几个组件，不要先加了再删」
