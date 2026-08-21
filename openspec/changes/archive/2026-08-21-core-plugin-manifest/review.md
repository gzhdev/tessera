# 代码审查报告 — core-plugin-manifest（未提交工作区变更）

- **审查日期**：2026-08-21
- **审查对象**：工作区未提交变更（本变更的实施内容：tessera-manifest crate 完整实现——清单类型定义、JSON Schema 生成工具、12 步校验流水线、依赖图解析，586 行生成物 `schema/plugin.schema.json`，CI 新增 schema 一致性检查步骤，规划件回填）
- **审查基准**：`AGENTS.md`（工作区规范）、`plugin-core-architecture-design.md` v0.3（权威设计书）、本变更的 `proposal.md` / `design.md` / `tasks.md` / `specs/`
- **审查方式**：5 路独立审查（规范符合 / 浅层 bug / 历史上下文 / openspec 验收 / 注释符合），发现的问题经逐条复核评分（0–100 置信度），≥80 为必须报告阈值

## 结论

**无 ≥80 置信度的必须报告问题。** 共发现 9 个问题，最高 75 分，全部低于阈值。

变更整体质量高（多个审查代理实跑验证，非仅读码）：

- **架构红线全部守住**：schema 生成物与类型定义逐字节一致（`cargo run --bin gen-schema` 重新生成后 `git diff --exit-code` 零差异，CI 已加一致性检查步骤）；`deny_unknown_fields` 经 serde 探针测试（`tests/serde_probe.rs`）实证生效；网络权限三类型（`net.http` / `net.insecure` / `net.private`）独立无隐含；`engines.core` 整数能力级别与应用 SemVer 解耦；tessera-manifest 保持叶子地位（`cargo tree` 确认无 tauri / wasmtime / 内核 crate 依赖）
- **校验流水线与设计书 §4.4 真值表逐行对应**（步骤 1–10 在 `validate()`，11/12 归 `DepGraph::resolve()`，与 design.md D1 拆分一致）；错误码映射正确，上次审查补齐的 `E_MANIFEST_VERSION` / `E_MANIFEST_MAIN` 已被步骤 3/6 正确使用；上次的修复（fail-closed 脚本、JSON 转义加固）未被回退
- **依赖图实现正确**：迭代 Tarjan 的 lowlink 回传、SCC 环路径还原、Kahn 拓扑序与逆序停用、级联失败根因传递均有测试覆盖
- `cargo test -p tessera-manifest` 57 个测试全绿；`cargo test --workspace` 全绿；fixture 覆盖全部 14 个错误码且由遍历式测试自动核对；`full-example` fixture 与设计书 §4.1 逐字段一致
- tasks.md 27 项勾选（diff 中恰为 27 个 `[ ]`→`[x]`，任务描述零改动）全部有真实实现与验证支撑，无凑勾选痕迹

## 接近阈值（75 分）的问题——建议提交前处理

### 1. 多段局部名使 `QualifiedId::parse`/`split` round-trip 破坏

- **位置**：`crates/tessera-manifest/src/ids.rs:105-124`（拆分逻辑）；文档承诺见 `ids.rs:97-104`；round-trip 测试 `ids.rs:230-236` 只覆盖单段
- **问题**：`LocalName` 按设计书 §6.2 允许点分多段（如 `tools.format`），但 `QualifiedId::parse` 用 `rsplit_once('.')` 拆分、假设局部名是最后一段。实跑复现：`PluginId::parse("com.example").qualify(LocalName::parse("tools.format"))` 得 `com.example.tools.format`，再次 `parse` 后 `split()` 静默返回 `(com.example.tools, format)` 而非 `(com.example, tools.format)`——违背类型文档"必须能拆回合法的插件 id + 局部名"的承诺。附带发现：大写段局部名（`Tools.Format` 合法）拼出的全限定名会被 `parse` 直接拒绝（head 含大写不是合法插件 id）。`split()` 当前无生产调用方，但它是下游 change（lifecycle/registry、全局冲突检查）将依赖的契约 API，且错误是静默的。
- **修复方向**：协议层面约定拆分规则（如解析时优先匹配最长合法插件 id 前缀，或约定多段局部名只能整体匹配 contributes 键），并补多段局部名的 round-trip 测试。

### 2. 激活事件多段局部名悬空引用漏检（与问题 1 同根）

- **位置**：`crates/tessera-manifest/src/validate.rs:302-309`（`check_activation_events`）
- **问题**：引用解析逻辑为「不含点 → 查本插件 contributes；含点且不带本插件 id 前缀 → 一律放行为外部引用」。实跑复现：`activationEvents: ["onCommand:tools.format"]`（contributes 中无此命令）通过校验、未报 `E_MANIFEST_DANGLING_REF`——spec「激活事件必须语法合法且引用有效」的 SHALL 语句在合法输入形态下未执行。而 `tools.format` 头段只有一段、不可能是合法插件 id（须至少两段），不可能是合法外部全限定名，放行在语义上矛盾。后果是该错误延迟到激活期暴露（命令永远无法触发激活）而非装载期拒绝。
- **修复方向**：判定顺序改为「own 前缀 → 不含点 → 头部能拆出合法 PluginId（外部引用放行）→ 否则按本插件局部名查 contributes，查不到即报 `E_MANIFEST_DANGLING_REF`」。

### 3. proposal.md 依赖清单漏列 `serde_path_to_error`

- **位置**：`openspec/changes/core-plugin-manifest/proposal.md:41-43`（Impact「新增依赖」）
- **问题**：同批新增的 schemars、semver 都已列入，唯独漏了 `serde_path_to_error`（根 Cargo.toml workspace 依赖表 + `crates/tessera-manifest/Cargo.toml`）——而它正是实现 spec MUST 级要求「错误信息指出哪个上下文中的哪个未知字段」（JSON 路径前缀）的关键依赖。design.md D3 验证结果段已提及，proposal 未同步，OpenSpec 工件间不一致，会误导后续依赖审计。
- **修复**：proposal.md Impact 段补一行。

## 低优先级（50 分）问题——可顺手处理

### 4. `ActivationEvent` 序列化形状与反序列化不对称

- **位置**：`crates/tessera-manifest/src/manifest.rs:155`（derive `Serialize`）与 `manifest.rs:220-225`（手写 `Deserialize`）
- **问题**：derive 的 `Serialize` 输出 `{"onCommand":{"command":"run"}}`（externally tagged 形状），而手写 `Deserialize` 与生成 schema（`plugin.schema.json:94-98`，`"type": "string"` + pattern）只接受 `"onCommand:run"` 字符串。`PluginManifest` 派生了 `Serialize`，一旦未来序列化清单（SDK / 缓存 / round-trip 测试）再读回必然失败。同文件 `StrictVersion`/`SemverRange` 都手写了对称的字符串 `Serialize`，`to_raw()` 也已存在——修复约 4 行（手写 `Serialize` 调 `to_raw()`）。当前无任何代码路径序列化清单，故未升格。

### 5. fixtures 注释与实际行为不符（死文件）

- **位置**：`crates/tessera-manifest/tests/fixtures.rs:15-16`（模块注释）
- **问题**：注释声称「目录内的 `plugin.wasm` 是占位文件，满足步骤 6 的存在性检查」，实际 `valid_fixtures_pass_validation` 传入的是 `temp_plugin_dir()`（临时目录占位文件），fixture 目录内的两个 `plugin.wasm` 从未被读取，是死文件。测试逻辑本身正确，属注释撒谎 + 冗余文件；修注释或让 fixture 目录自身作 `plugin_dir` 均可。

### 6. `scope_key` 顺序敏感导致 scope 重复声明漏检

- **位置**：`crates/tessera-manifest/src/manifest.rs:120-134`
- **问题**：`Vec::join(",")` 构键不排序，`["a","b"]` 与 `["b","a"]` 键不同，语义上同一 scope 集合重复声明不会被查重（`manifest.rs:460-468`）捕获。设计书 §4.2「同一 `type` + scope 不得重复」未定义「scope 相等」是集合还是序列等价，当前实现属合法解读；且权限并集生效，漏检只是冗余不扩权。修复：排序去重后再 join，并补顺序变体测试。

### 7. `E_MANIFEST_VERSION` 错误文案偏离设计书范式

- **位置**：`crates/tessera-manifest/src/validate.rs:133`
- **问题**：文案「请更新插件」偏离设计书 §13.3（:1582）范式「请升级应用」，且 §13.3 明确建议动作只能是「升级应用 / 降级插件 / 联系插件作者」三种；同函数步骤 4（`E_COMPAT_CORE`，:144）正确遵循了范式，此处更像疏漏。manifestVersion 超前的语义是"插件为未来宿主所写"，应引导升级应用。spec 未逐字约束此文案，不影响功能，建议对齐。

### 8. CI 头注释封闭列举未随新增步骤更新

- **位置**：`.github/workflows/ci.yml:3-4`（头注释）与 `:17`（job name）
- **问题**：本次新增了第 5 类步骤「schema 生成物一致性（§4.3）」（:43-46），但头注释「只要 fmt / clippy / test / 两条依赖方向检查 / 一个 WASM 组件往返冒烟」未同步，描述失实。补一词即可。

### 9. `StrictVersion` pattern 比 semver 严格校验宽松

- **位置**：`crates/tessera-manifest/src/schema.rs:26-31`
- **问题**：pattern 接受 `01.02.03`、`1.0.0--`、`1.0.0-01` 等会被 `semver::Version::parse` 拒绝的形式，未完全达成自述目标「使补全提示与实际校验规则一致」。但 schema 已明确声明仅为编辑器补全、不得用作校验入口（校验真值在 serde 类型），影响仅限畸形输入不弹提示，合法值不会被误拦。ECMA-262 正则完整表达 semver 严格规则成本较高，可接受现状或收紧 pattern。
