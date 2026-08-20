# core-plugin-manifest

## Why

插件清单是宿主认识插件的唯一入口。§3.4 的懒激活依赖「不执行任何插件代码就能把 UI 搭出来」，这条能力的全部信息来源就是 `plugin.json`——清单解析的质量直接决定启动性能与用户能否在插件激活前看到菜单。

清单校验同时是插件作者最先接触到的界面。§4.3 明确要求「插件作者拼错字段名时要立刻得到反馈，而不是纳闷为什么我的配置没生效」，这意味着错误信息的精确度是功能需求，不是打磨项。

这块工作**完全不依赖 wasmtime**，可以在工具链验证进行的同时并行开工，且能用 fixture 文件独立验收——它是新接触 Rust 的团队最合适的第一块实心工作。

## What Changes

- 清单的 Rust 类型定义（§4.2 逐字段规范），`serde` 反序列化 + `deny_unknown_fields`
- **校验真值在 Rust 侧**（v0.3 决策 A2）：`plugin.schema.json` 改由 `schemars` 生成并 check-in，仅服务编辑器补全，不作为校验入口
- §4.4 校验流水线中不需要全局上下文的步骤：清单解析、字段规范、`manifestVersion`、`engines.core`（整数能力级别比较）、`engines.pluginApi`（SemVer 区间）、`main` 路径合法性、`activationEvents` 语法与悬空引用、虚拟目录别名存在性
- 局部名 ↔ 全限定名的拼接与解析规则（§4.2）
- 依赖图：缺失/版本检查、Tarjan 强连通分量环检测（错误信息含完整环路径）、拓扑排序
- 全部 `E_MANIFEST_*` / `E_COMPAT_*` / `E_DEPS_*` 错误码的产出，每个至少一条反例 fixture

**范围边界**（重要）：§4.4 的第 7/8 步（全局 id 冲突）需要 `ExtensionRegistry`，第 11/12 步的**编排**需要跨插件视图。本 crate 只提供**纯函数与依赖图算法**，不持有全局状态；跨插件冲突检查的编排归 `core-plugin-lifecycle`。否则 `tessera-manifest` 会反向依赖 `tessera-core`，违反 §17.1 的依赖方向。

## Capabilities

### New Capabilities

- `plugin-manifest`：清单的结构、字段规范、解析与校验规则，以及校验失败时的错误码与信息质量
- `plugin-dependency-graph`：插件间依赖的存在性与版本检查、环检测、加载与停用顺序的推导

### Modified Capabilities

（无）

## Impact

**新增**

- `crates/tessera-manifest/src/{manifest.rs, validate.rs, depgraph.rs, ids.rs}`
- `crates/tessera-manifest/schema/plugin.schema.json`（**生成物**，纳入 git，CI 校验一致性）
- `crates/tessera-manifest/tests/fixtures/`：一份正例（§4.1 的完整示例）+ 每个错误码至少一条反例

**新增依赖**

- `serde` / `serde_json`、`schemars`、`semver`

**下游影响**

- `core-plugin-lifecycle` 消费本 crate 的类型与依赖图算法完成 §4.4 的全流水线
- `core-permission-model` 消费 `Permission` 枚举装配能力
- `core-plugin-sdk` 的模板产出必须能通过本 crate 的校验

**风险**

- serde 的 internally tagged enum（`#[serde(tag = "type")]`）在分发前会缓冲 Content，历史上与 `deny_unknown_fields` 的组合存在行为不一致。整个清单校验的错误信息质量都押在它上面。**第一个测试用例就必须是未知字段的反例**，确认它真的报错；若不生效，退路是给每个 variant 载荷单独标注或改用 adjacently tagged
