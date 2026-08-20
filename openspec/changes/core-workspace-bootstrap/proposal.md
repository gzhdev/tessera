# core-workspace-bootstrap

## Why

设计书 `docs/plugin-core-architecture-design.md` v0.3 已把架构收敛到可开工粒度，但仓库里目前只有一个空的 `src/main.rs`。开工前需要一个能承载后续 19 个 change 的工程基座。

这个基座里有一件事必须最先做：**验证 WASM Component Model 工具链真的能跑通**。设计书 §1.3 自己承认「工具链相对新，需要跟进 wasmtime 版本」，而整个架构的存亡都押在这条假设上——§8 的沙箱、§5 的 WIT 契约、§14 的 SDK 全部由它派生。证伪成本是 1–3 天；在 workspace 结构与 WIT 定型之后才发现问题，返工成本是数周。

同时，§17.1 的两条硬性约束（`tessera-core` 不得依赖 wasmtime、五个内核 crate 不得依赖 tauri）若不在第一天就由 CI 强制，半年后会变成无法回收的技术债——依赖是悄悄长进来的，等发现时已经缠满整棵树。

最后，错误码与日志是横切设施。不先立起来，各 crate 会各自定义 `Error` 类型和各自的日志约定，后面统一是一次波及全仓库的重构。

## What Changes

- **工具链基线验证**：最小 WIT 往返（1 个 import + 1 个 export）→ `wit-bindgen` guest → `wasm32-wasip2` → `wasm-tools component new` → wasmtime 宿主实例化并回调 host 函数。产出 `docs/toolchain-baseline.md`（可复现的版本组合），并把往返用例保留为 CI 冒烟测试，用于防工具链漂移
- **Cargo workspace 与 crate 骨架**：`tessera-manifest` / `tessera-ui-schema` / `tessera-store` / `tessera-sandbox` / `tessera-core` / `tessera-sdk` / `src-tauri`，依赖边按 §17.1 的方向图连好
- **前端脚手架**：Vite + Vue 3 + TypeScript，`tauri dev` 能起窗口并渲染一个占位页
- **依赖方向的 CI 强制**：§17.1 约束 1（五个内核 crate 依赖树中不得出现 `tauri`）与约束 2（`tessera-core` 依赖树中不得出现 `wasmtime`）
- **统一错误模型**：§12.1 的全部错误码前缀落为 Rust enum；三段式错误结构（错误码 / 面向用户的一句话 / 面向开发者的细节）
- **可观测性基座**：`tracing` + `tracing-subscriber`，按天轮转、保留 7 天、强制 `plugin_id` span、单条 8 KB 截断、单插件 100 条/秒限速

**非目标**（明确不在本 change 内）：任何清单解析逻辑（见 `core-plugin-manifest`）、任何 SQLite 访问（见 `core-persistence`）、任何真实的插件加载（见 `core-wasm-sandbox`）。本 change 结束时 crate 是空壳，但骨架、约束与横切设施就位。

## Capabilities

### New Capabilities

- `error-model`：统一的错误码分类与三段式错误结构，规定错误如何被分类、序列化、以及哪一段进 UI、哪一段进日志
- `observability`：日志的输出目标、轮转与保留策略、插件日志的隔离与限速
- `build-constraints`：crate 依赖方向的硬性约束，以及生成物与 check-in 内容一致性的校验规则

### Modified Capabilities

（无——这是本项目的第一个 change，`openspec/specs/` 目前为空）

## Impact

**新增**

- `Cargo.toml`（workspace 根）、`crates/*/`（7 个 crate 骨架）、`src-tauri/`、`src/`（前端脚手架）
- `docs/toolchain-baseline.md`
- `.github/workflows/ci.yml`（或等价 CI 配置）
- `examples/toolchain-roundtrip/`（工具链冒烟测试）

**修改**

- 现有的 `Cargo.toml` 从单 package 改为 workspace；`src/main.rs` 移入 `src-tauri/`

**新增依赖**

- `thiserror`（错误类型）、`tracing` / `tracing-subscriber` / `tracing-appender`（日志）
- 验证阶段：`wasmtime`、`wit-bindgen`、`wasm-tools`（前两者最终落在 `tessera-sandbox`，本 change 只在 `examples/` 中用到）

**下游影响**

- 全部 19 个后续 change 都从这里的 crate 骨架与错误模型起步
- §17.1 的两条 CI 检查会在任何人误加依赖时立刻变红，这是刻意的摩擦

**风险**

- 最高风险集中在工具链验证：若 `wasm32-wasip2` + component model 的工具链组合在 Windows 上不可用或极不稳定，§8 的整个沙箱方案需要重新评估。这正是把它排在第一位的原因
