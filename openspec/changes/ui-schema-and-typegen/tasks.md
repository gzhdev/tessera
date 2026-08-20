# ui-schema-and-typegen · 任务

> 推进顺序按 design.md 的 Migration Plan：先用 5–6 个代表性组件把生成管线与一致性检查跑通（第 1–3 组），再批量补齐其余组件（第 4 组）。先定义完 30 个组件再验证管线，是把风险堆到最后。

## 1. 生成器选型验证（阻塞性）

- [ ] 1.1 用一个带枚举 props 的组件与一个带嵌套结构的组件试跑 `ts-rs` 生成。验证：产出的 TS 类型能被前端 `tsc` 接受，且联合类型的判别字段可用于收窄
- [ ] 1.2 若 1.1 产出不可用，按 design.md D1 的退路调整 Rust 侧类型表达（而非换生成器）。验证：调整后 1.1 的判定通过
- [ ] 1.3 选定生成命令的载体（倾向 `cargo xtask`，见 design.md Open Questions）。验证：一条命令可重复执行且产出稳定

## 2. 节点结构与代表性组件

> 对应 `specs/ui-schema/spec.md` 的节点结构类 Requirement

- [ ] 2.1 定义节点结构：`id`、组件标记枚举（type + props）、`children`、`on`（design.md D2）。验证：只含 `type` 与 `id` 的最小节点解析成功
- [ ] 2.2 定义 6 个代表性组件的 props 契约：`vstack`（容器）、`hstack`（容器）、`text`（纯展示）、`button`（无值事件）、`text-input`（有值事件）、`table`（结构化载荷事件）。验证：每个组件各有一条正例 fixture 解析成功
- [ ] 2.3 实现 `on` 映射的透传语义：动作字符串不被解释或校验。验证：任意自定义动作名解析成功并在结果中原样保留
- [ ] 2.4 定义交互事件结构（`nodeId` / `event` / `action` / `value`）。验证：无值交互的 `value` 为空；带值交互按组件契约携带载荷；表格行交互同时携带行序号与行数据

## 3. 校验、降级与生成管线

- [ ] 3.1 实现两段式反序列化（design.md D4）：先用宽松中间表示解析整树，再逐节点尝试强类型解析。验证：含未知类型的树整体解析成功，不因单个未知节点整树失败
- [ ] 3.2 实现 `ValidationOutcome`（树 + 降级清单 + 警告清单，design.md D3）。验证：10 个节点中 1 个未知时，降级清单恰含该节点，另 9 个正常，产生警告而非错误
- [ ] 3.3 实现未知类型节点的子树一并降级。验证：未知节点带 `children` 时整体降级，子树不被单独渲染
- [ ] 3.4 实现同树内重复 `id` 的检出与警告，警告含重复值与位置。验证：两个节点同 `id` 时产生可定位的警告
- [ ] 3.5 实现属性类型不符的报告。验证：数值型属性被赋字符串时，校验指出该节点的该属性类型不符
- [ ] 3.6 接通生成管线，产出 `src/types/generated/` 并 check-in，文件内标注「自动生成，勿手改」。验证：前端能仅凭生成的类型完成一次节点构造，无需手写任何契约类型
- [ ] 3.7 在 CI 中加入一致性检查。验证：改了 Rust 类型未重新生成时 CI 失败；直接手改生成产物时 CI 也失败

## 4. 补齐组件清单

> 对应 `specs/ui-schema/spec.md` 的组件清单 Requirement。按 design.md Risks 第二行：把握不准的组件优先砍掉——加组件是兼容变更，删组件要 `CORE_API_LEVEL` +1

- [ ] 4.1 补齐布局类其余组件：`grid`、`scroll`、`tabs`、`group`、`spacer`、`split`。验证：各有正例 fixture 且解析成功
- [ ] 4.2 补齐展示类：`heading`、`badge`、`divider`、`icon`、`markdown`、`code`、`image`、`empty-state`。验证：同上
- [ ] 4.3 补齐输入类：`textarea`、`number-input`、`select`、`checkbox`、`radio-group`、`switch`、`slider`。验证：同上
- [ ] 4.4 补齐数据类与反馈类：`list`、`tree`、`key-value`、`alert`、`progress`、`spinner`。验证：同上
- [ ] 4.5 定义 `file-picker` 及其事件载荷 `{ token, fileName, size }`，`token` 为不透明字符串（design.md D5）。验证：载荷中不存在任何路径字段；一个未声明文件系统权限的场景下该组件的契约仍完整可用
- [ ] 4.6 确认 `TextInputProps` 中不存在 password / masked 字段（design.md D6）。验证：尝试为文本输入声明掩码属性时校验失败，因为该属性不在契约中
- [ ] 4.7 每次补齐后重新生成 TS 类型并提交。验证：CI 一致性检查全绿

## 5. 收尾

- [ ] 5.1 建立 fixture 目录并补齐覆盖：全组件树、嵌套容器树、未知类型树、重复 id 树、非法 props 树。验证：每条 spec 场景至少对应一个 fixture
- [ ] 5.2 编写「新增一个组件」的操作说明，明确顺序为「改类型 → 重新生成 → 实现渲染」。验证：按说明走一遍能完整加入一个新组件，且中途任一步骤遗漏都会被工具挡住
