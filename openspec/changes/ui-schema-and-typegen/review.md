# 代码审查报告 — ui-schema-and-typegen（提交 f8a317c）

- **审查日期**：2026-08-21
- **审查对象**：提交 `f8a317ca3e69f73d9b9abb4e00de76695824a077`（本变更的实施内容：tessera-ui-schema crate——UiNode 节点结构、34 个组件 props 契约、UiEvent 事件、两段式反序列化 + ValidationOutcome 降级清单、ts-rs 生成管线产出 `src/types/generated/` 51 个 .ts 并 check-in、CI 一致性检查步骤、fixtures 体系与 51 项测试）
- **审查基准**：`AGENTS.md`（工作区规范）、`plugin-core-architecture-design.md` v0.3（§9 UI 描述树、§17.1 约束 5）、本变更的 `proposal.md` / `design.md` / `tasks.md` / `specs/`
- **审查方式**：5 路独立审查（规范符合 / 浅层 bug / 历史与教训重演 / OpenSpec 验收 / 注释文档），去重后 15 个候选问题，逐条独立评分（0–100 置信度），≥80 为必须报告阈值

## 结论

**发现 1 个 ≥80 置信度的必报告问题（100 分）**——本系列四次审查（core-workspace-bootstrap、core-plugin-manifest、core-persistence、本次）首次触发必报告阈值。另有 3 个 75 分、11 个 50 分，全部低于阈值。

变更整体质量仍然扎实（多代理独立实跑验证）：

- **架构红线全部守住**：生成物 check-in 且「重新生成 + `git diff --exit-code`」实测零差异；tessera-ui-schema 依赖树无 tauri（check-deps 7 项 PASS）；组件清单恰好 34 个且与设计书 §9.2 五类清单逐项一致（`all-components.json` 程序化比对 missing/extra 均为空）；`file-picker` 载荷 `{token, fileName, size}` 无路径字段（`deny_unknown_fields` 实证）；`TextInputProps` 无 password / masked；ts-rs 选型与 design.md D1 及设计书 §17.1 约束 5 明文一致
- **OpenSpec 验收**：ui-schema spec 7 Requirement / 13 Scenario 中 12 个实测通过，唯一不通过的是「最小合法节点」（问题 1）；ui-typegen spec 4 Requirement / 5 Scenario 全部通过；design.md D1–D6 决策均被遵守（D3 降级清单非 Result、D4 两段式、D5 载荷定死、D6 password 类型缺席）
- `cargo test -p tessera-ui-schema` 51 项全绿；clippy / fmt / 依赖方向 / 日志检查 / `openspec validate` 全部通过；前三变更的全部 FIX 修复无回退

主要缺口集中在三处：**最小节点 `props` 缺省处理**（问题 1，spec MUST 级 + 任务虚验）、**探针残留**（问题 2）、**降级边界的整树连坐**（问题 3）。

## 必报告问题（100 分）——归档前必须修复

### 1. spec「最小合法节点」场景未实现：省略 `props` 即整节点降级，且 tasks 2.1 属虚验

- **位置**：`crates/tessera-ui-schema/src/validate.rs:182-184`（仅当 `raw.props` 存在时才向 `node_json` 插入 `props` 键）；`crates/tessera-ui-schema/src/node.rs:56`（`#[serde(tag = "type", content = "props")]` 邻接标记枚举）；`crates/tessera-ui-schema/tests/positive.rs:73-81`（虚验测试）
- **问题**：spec 明文「`type` 与 `id` MUST 存在；**其余字段可省略**」，Scenario「最小合法节点：WHEN 节点只有 `type` 与 `id` THEN 该节点合法」。但 serde 邻接标记枚举在 content（`props`）字段缺省时反序列化直接失败（`missing field 'props'`），节点被降级并误报 `PropTypeMismatch`。实跑（评分代理仓库外探针）：`{ "root": { "type": "vstack", "id": "x" } }` → `usable=false, degraded=1`；`spacer` / `button` / `text` 最小节点同样全部失败；最小节点为根时整树 `tree=None`。对照组：同一节点带 `"props": {}` 即通过——修复（缺省补空对象）极简单但未做。TS 侧 `src/types/generated/UiNode.ts` 34 个判别分支 `props` 也全部必填，两端一致拒绝 spec 允许的形态
- **虚验**：测试名 `minimal_node_only_type_and_id` 实际输入带 `"props": { "text": "x" }`，名不副实；tasks.md 2.1 以该测试作为「只含 type 与 id 的最小节点解析成功」的验收证据——违反 AGENTS.md「实现后必须逐条实际验证，不可只靠编译通过」
- **修复方向**：`resolve_node` 在 `props` 缺省时插入 `"props": {}`（props 全可选的组件即恢复合法；有必填 props 的组件仍会正确报属性缺失）；重写 `minimal_node_only_type_and_id` 为真最小节点断言；TS 侧 `props` 是否标可选需与 spec 口径对齐后同步生成

## 接近阈值（75 分）的问题——建议同批处理

### 2. `TextProps.extra_probe` 选型验证探针残留进入正式契约

- **位置**：`crates/tessera-ui-schema/src/components.rs:54-56`；生成物 `src/types/generated/TextProps.ts`（`extraProbe?: number`）
- **问题**：三个审查代理独立命中。全仓库零引用、无 doc comment（components.rs 83 个 pub 字段中唯一裸字段），对应 tasks 1.1/3.6 的 tsc 探针残留（tasks 5.2 对临时组件 `pill` 的「验证后已移除」纪律未覆盖到此）。已随管线导出进生成物；契约受 `CORE_API_LEVEL` 约束，发布后删除即构成不必要的契约变更——当前未发布，删除零成本
- **修复**：删除该字段并重新生成（生成物 diff 即修复证据）。

### 3. RawNode 强类型字段使后代形状错误整树连坐，违背 design.md D4「宽松中间表示」

- **位置**：`crates/tessera-ui-schema/src/validate.rs:80-91`（`RawNode` 的 `id: Option<String>`、`on: Option<HashMap<String, String>>`、`children: Vec<RawNode>` 均强类型）与 `:107-132`（第一阶段失败即 `tree: None`）
- **问题**：两个审查代理独立实跑复现。10 节点树中仅一个孙节点 `"id": 5`（或 `on` 值非字符串、或缺 `type`）→ 整树 `is_usable=false, degraded=0`，正常兄弟全部连坐、前端连错误占位符都无处渲染；而同一字段更轻的「缺 id」反而正确节点级降级（`Option` 能过第一阶段，第二阶段由 `UiNode` 必填检出）——行为不一致是强类型化的偶然产物而非政策。design.md D4 明文「宽松的中间表示（`type: String` + `props: RawValue`）」、Risks 第三行明文「中间表示只有两个字段」，与实现不符；错误消息还误导性地报在 `root` 路径。输入源正是不受信插件返回的 JSON（core-ui-bridge 场景）。既有测试只覆盖未知 `type` 与 props 不符，未覆盖 id/on/children 形状错误
- **修复方向**：`id` / `on` 宽松化为 `serde_json::Value` 延迟到第二阶段解析，失败即走既有节点级降级路径，并补形状错误降级测试。

### 4. `FilePickValue.size` 生成 TS 类型为 `bigint`，与 JSON 运行时的 `number` 不符

- **位置**：`crates/tessera-ui-schema/src/event.rs:77`（`pub size: u64`）；`src/types/generated/FilePickValue.ts:24`（51 个生成物中唯一 `bigint`）；`crates/tessera-ui-schema/src/bin/gen-types.rs`（未设 `TS_RS_LARGE_INT`）
- **问题**：ts-rs 12 对 `u64` 默认映射 `bigint`（本地 registry 源码确认，仅 `TS_RS_LARGE_INT` 环境变量可覆盖），而 serde_json 把 `u64` 序列化为 JSON number、前端 `File.size` 运行时也是 number。pick 事件的生产者是宿主前端：按生成类型把 `size` 写成 bigint 后 `JSON.stringify` 直接抛 TypeError；绕过类型则契约失效。CI 一致性检查已把错误形态固化
- **修复**：`gen-types` 设 `TS_RS_LARGE_INT=number`，或该字段加 `#[ts(type = "number")]`。

## 低优先级（50 分）问题——可顺手处理或留档

### 5. CI 头注释与 job name 未随新增步骤更新（教训第二次重演）

- **位置**：`.github/workflows/ci.yml:4`（头注释）、`:17`（job name）、`:48-51`（新增步骤）
- **问题**：plugin-manifest review.md 问题 8 的精确重演——2775cd4 刚把头注释修准确，本提交新增第 5 类检查「UI 类型生成物一致性」后再次失实；且 job name 从未被同步过。纯注释失实、无功能影响，但同一模式连续两变更复发，说明封闭列举维护纪律未内化。
- **修复**：补词，或将头注释改为非封闭列举式表述以绝后患。

### 6. proposal.md 依赖清单漏列 `serde_json`

- **位置**：`openspec/changes/ui-schema-and-typegen/proposal.md:44`；实际新增见 `crates/tessera-ui-schema/Cargo.toml`
- **问题**：前次问题 3（漏列 `serde_path_to_error`）同型重演：清单列了 ts-rs、serde，漏了同为校验核心的 `serde_json`。附带新发现：`thiserror` 在本 crate 全部 .rs 中零引用——正确处置是从 Cargo.toml 删除而非补入 proposal。
- **修复**：proposal 补列 `serde_json`；删除未使用的 `thiserror` 依赖。

### 7. 警告分类依赖 `contains("unknown variant")`，嵌套枚举未知值被误标

- **位置**：`crates/tessera-ui-schema/src/validate.rs:207-215`
- **问题**：实跑复现 `button` 的 `variant: "weird"` 被误标为 `UnknownComponent`（serde 对邻接枚举未知 type 与 props 内枚举未知值措辞相同）；缺 id / 缺 props 等结构性错误一律归 `PropTypeMismatch`；整体依赖 serde 英文文案，升级改措辞即静默失效。spec 字面 Scenario（数值型属性赋字符串）分类正确，降级行为两类一致，故 50 分。
- **修复方向**：第二阶段先对 `type_tag` 做白名单判别，或引入结构化错误判别（如 `serde_path_to_error`）。

### 8. CI 无前端类型检查步骤，拦截表「TS 类型检查」一点仅本地生效

- **位置**：`crates/tessera-ui-schema/ADDING_A_COMPONENT.md:41`；`.github/workflows/ci.yml`（仅 rust-checks 与 toolchain-smoke 两个 job）
- **问题**：拦截表中两个 `git diff --exit-code` 均为 CI 强制，独「TS 类型检查」只在本地 `npm run build` 生效。但 spec Scenario 字面由类型系统满足（spec 有意识区分「CI 校验」与「类型检查」两种措辞）、被拦截的前端代码尚不存在（属 ui-component-library 范围），归下一变更的 CI 任务合理。
- **修复方向**：ui-component-library 引入前端代码时在 CI 增加 vue-tsc 步骤。

### 9. `EventValue` untagged 中 `Selection(String)` 反序列化不可达

- **位置**：`crates/tessera-ui-schema/src/event.rs:29`（注释「untagged，由结构区分」）、`:34-50`（`Text(String)` 声明在 `Selection(String)` 之前）
- **问题**：serde untagged 按序尝试，任何 JSON 字符串载荷都解析为 `Text`，`Selection` 变体反序列化不可达；TS 生成物两个 `string` 无法区分。探针实证成立，但 spec 未要求类型层可区分、载荷字符串值本身无损、当前无生产反序列化路径。
- **修复方向**：合并同形变体或调整注释承诺，避免误导下游按变体分发。

### 10. 模块文档仍描述「六个代表性组件」中间状态

- **位置**：`crates/tessera-ui-schema/src/components.rs:1-5`、`node.rs:8-9`
- **问题**：两处模块头注释称「先落 6 个代表性组件……其余组件见第 4 组任务」，实际 34 个已全部落地（第 4 组任务已勾选）。前次「注释与实际不符」同型模式第二次出现。
- **修复**：按最终状态改写两处注释。

### 11. proposal 路径 `components/*.rs` 与 design D2「按类别分模块组织」未兑现

- **位置**：`openspec/changes/ui-schema-and-typegen/proposal.md:37`、`design.md:47`；实际为单文件 `components.rs`（630 行）
- **问题**：工件声明多文件目录，实现为单文件；工作区还残留空的 `components/` 目录佐证中途改向未回写。D2「分模块」位于代价段非决策本体，单文件对单一枚举反而更直接，无功能影响。
- **修复方向**：回写工件措辞（或择机拆分模块并删除空目录）。

### 12. tasks 1.1/3.6 的 tsc 探针未注明「临时工件已删除」

- **位置**：`openspec/changes/ui-schema-and-typegen/tasks.md:8`、`:27`
- **问题**：全仓库无 `@ts-expect-error` 或探针 .ts；5.2 对 `pill` 明确注明「验证后已移除」，1.1/3.6 未注明，验收证据不可直接复核（核心主张可由 `vue-tsc --noEmit` exit=0 间接复现）。
- **修复**：3.6 补一句「探针为临时工件，验证后已删除」。

### 13. `ImageProps.source` 注释引入设计书未定义的「blob 标识」概念

- **位置**：`crates/tessera-ui-schema/src/components.rs:333`
- **问题**：设计书 §9.6 实为「`image` 组件接受 `data:` URI 与虚拟路径」，全文无「blob」；注释自造概念且「不含文件系统路径」无机制支撑（`source` 为自由 String）。已随 ts-rs 传播进 `ImageProps.ts` 的 JSDoc。
- **修复**：注释对齐设计书 §9.6 措辞后重新生成。

### 14. 非容器组件携带 `children` 被静默接受，`is_container()` 校验路径零引用

- **位置**：`crates/tessera-ui-schema/src/node.rs:210-221`（仅测试引用）
- **问题**：实跑 `text` 带 children → `usable=true, degraded=0`。spec 的「children（仅容器类组件有）」是括号说明、无 Scenario 强制，宽容策略可辩护；但超契约输入零反馈，插件作者会困惑。
- **修复方向**：可选——非容器携带 children 时产生一条警告。

### 15. CI 一致性检查检不出「孤儿生成物」

- **位置**：`crates/tessera-ui-schema/src/bin/gen-types.rs`（只覆盖写不清理）；`.github/workflows/ci.yml:48-51`
- **问题**：临时 git 仓库实证——类型删除/改名后残留的 `.ts` 不会被重新生成触碰，`git diff --exit-code` 退出码 0，CI 静默通过；前端 import 死类型也不报错。spec 两个 Scenario 均未承诺删除场景，属 spec 与实现的共同盲区，触发条件在未来。
- **修复方向**：gen-types 生成前清空导出目录（或 CI 改为「删除目录→重生成→diff 全目录」）。

---

**提交建议**：问题 1 为 spec MUST 级违约 + 验收虚验，归档前必须修复；问题 2/3/4 修复成本均低（2 删一行、3 宽松化两个字段、4 加一个注解或环境变量），建议与问题 1 同批落地 `FIX:` 提交并重新生成类型；50 分项可顺手处理或在后续变更中消化。
