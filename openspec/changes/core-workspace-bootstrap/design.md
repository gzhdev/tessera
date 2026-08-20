# core-workspace-bootstrap · 设计

## Context

见 `proposal.md` 的 Why。当前仓库只有一个空的 `src/main.rs` 与一份单 package 的 `Cargo.toml`。

约束来自设计书 §17.1：五个内核 crate 不得依赖 Tauri，`tessera-core` 不得依赖 wasmtime。这两条不是建议，是「将来能换 UI 外壳」和「将来能把某类插件挪到进程外」两条演进路径的前提。

## Goals / Non-Goals

**Goals**

- 在写任何业务代码之前确认工具链可用，并把这个确认固化为可复现的 CI 用例
- 让 §17.1 的约束在第一天就有牙齿
- 让错误与日志的形状先定下来，避免各 crate 各自发明

**Non-Goals**

- 不做任何清单解析、数据库访问、插件加载。crate 在本 change 结束时是空壳
- 不追求 CI 的完备性（覆盖率、跨平台矩阵）。只要 fmt / clippy / test / 两条依赖检查 / 一个往返冒烟

## Decisions

### D1 · 工具链验证保留为 CI 冒烟测试，而非一次性丢弃

**选择**：往返用例落在 `examples/toolchain-roundtrip/`，进 CI 常驻。

**理由**：工具链验证的一次性价值是「现在能不能跑」，但它的长期价值更高——wasmtime、wit-bindgen、wasm-tools、Rust toolchain 四者中任一升级都可能破坏组合。常驻用例让漂移在升级当天暴露，而不是等到某个开发者本地构建失败才发现。增量成本几乎为零（用例已经写了）。

**放弃的选项**：写完删掉，只留一份 `docs/toolchain-baseline.md`。文档会过期，用例不会。

### D2 · 依赖方向检查用 `cargo tree -i` 而非 `cargo-deny`

**选择**：CI 中对每个受约束的 crate 跑 `cargo tree -i <禁止的包> -p <crate>`，有输出即失败。

**理由**：`cargo-deny` 的 `bans` 功能是全局的——它擅长「整个 workspace 不许出现某个包」，而这里需要的是「A 不许，B 允许」这种按 crate 的定向禁止。用 `cargo tree -i` 反查更直接，且输出天然就是引入路径，正好满足 spec 里「指出通过哪条路径引入」的要求。

**放弃的选项**：`cargo-deny`（表达力不匹配）；自己解析 `cargo metadata`（重复造轮子）。

**已知弱点**：`cargo tree` 默认按当前 feature 解析。若某依赖只在特定 feature 下引入 Tauri，默认检查会漏掉。缓解：检查命令加 `--all-features`。

### D3 · 错误类型用单一 enum + `thiserror`，不搞每 crate 一套

**选择**：错误码 `ErrorCode` 是一个全局 enum（放在最底层的共享位置），三段式结构 `TesseraError { code, user_message, developer_detail }` 同样只有一份。

「最底层的共享位置」在实施时定死为**新增第 8 个 crate `crates/tessera-error`**：§12.1 的 `E_HOST_DB` 必须由 `tessera-store` 构造，而 store 在 §17.1 方向图中是叶子——枚举放进任何现有内核 crate 都会产生图上不存在的依赖边。`tessera-error` 成为全部内核 crate 的下游新叶子（不破坏方向图），tauri/wasmtime 依赖检查把它一并纳入受检清单。

**理由**：spec 要求错误码可跨边界传递且可与常量做相等比较。若每个 crate 定义自己的 error 再层层 `From` 转换，跨边界时要么丢失原始错误码，要么每层都要维护映射表。单一 enum 的代价是底层 crate 会「认识」上层的错误码（比如 `tessera-manifest` 能构造 `E_TRAP_*`），这是可接受的——错误码是**契约**，不是实现细节。

**放弃的选项**：每 crate 一个 enum + `#[from]` 链（映射维护成本高、跨边界易丢信息）；`anyhow`（丢失结构化错误码，与 spec 冲突）。

### D4 · 日志限速做成 `tracing` 的 Layer，而非在写入点各自判断

**选择**：限速与截断实现为一个 `tracing_subscriber::Layer`，按 span 中的插件标识分桶。

**理由**：spec 要求限速按插件独立、且不影响宿主日志。若在 `host-log` 的实现里做判断，只能覆盖插件主动写的日志，覆盖不了宿主为插件写的日志（后者同样带插件标识、同样可能被插件的行为放大——比如一个疯狂触发权限拒绝的插件）。做成 Layer 则一处覆盖全部路径。

**放弃的选项**：在 `host-log` 的 host 函数里判断（覆盖不全）。

**实施注记**（落地形态与「Layer」的偏差）：限速实现为 per-layer `Filter`（在 `event_enabled` 阶段裁决，按事件 `plugin_id` 字段分桶）；截断落在自定义轮转写入器里（单条 8 KB 上限对宿主与插件一视同仁，属安全超集）；「进入丢弃状态记一条警告」因 tracing 的 per-layer FilterState 两阶段协议不允许在过滤器回调内再发事件，改为**旁路直写**日志文件（同构 JSON 行，不可能被限速自身丢弃）。

### D5 · crate 放在 `crates/` 子目录，`src-tauri` 与 `src` 留在根

**选择**：沿用设计书 §17 的布局原样。

**理由**：`src-tauri` 与 `src` 的位置是 Tauri 工具链的约定，动它会持续与工具链摩擦。内核 crate 集中在 `crates/` 下便于对它们统一施加 §17.1 的检查（检查脚本可以直接遍历该目录）。

### D6 · 前端脚手架手搭，不用 `create-tauri-app`

**选择**：手动配置 Vite + Vue 3 + TS 与 Tauri 集成。

**理由**：官方模板会生成一套自己的目录约定与示例代码，而设计书 §17 已经规定了 `components/ui`、`components/shell`、`types/generated`、`stores` 的分层。用模板后要先删一遍再改，不如直接按目标结构搭。前端脚手架本身的复杂度很低。

## Risks / Trade-offs

| 风险 | 缓解 |
|---|---|
| `wasm32-wasip2` 工具链在 Windows 上不可用或极不稳定 | **已证伪为可行**：往返用例通过，验证过的版本组合与构建命令见 `docs/toolchain-baseline.md` |
| 设计书 §8.2 把 `StoreLimits::instances` 定为 1，但 component model 下一次实例化可能产生多个 core instance，会导致合法插件装不起来 | **已实测命中**：最小可行值为 **3**（探针用例常驻 `examples/toolchain-roundtrip`，结论见 `docs/toolchain-baseline.md`）。`core-wasm-sandbox` 不得沿用 1，建议默认 8 |
| `cargo tree` 的 feature 敏感性导致假阴性 | 检查加 `--all-features`；并在 `core-wasm-sandbox`（第一个真正引入 wasmtime 的 change）中补一次人工复核 |
| 单一错误码 enum 会随项目增长变得很大 | 接受。错误码本来就是一份需要集中查阅的清单，分散反而更难维护。按域分模块组织即可 |

## Migration Plan

1. 把现有 `Cargo.toml` 改为 workspace 根，`src/main.rs` 移入 `src-tauri/src/`
2. 建 8 个 crate 骨架（含 `tessera-error`），依赖边按 §17.1 的方向图连好（此时都是空 lib，依赖边为空也合法——检查脚本此刻恒过，这是预期的）
3. 工具链验证独立于 workspace 推进，产出基线文档后再把冒烟用例接进 CI

无回滚需求——本 change 之前仓库几乎是空的。

## Open Questions

- CI 平台尚未选定（GitHub Actions / 其他）。这不影响任何 spec 或任务拆分，检查命令本身与平台无关，选定后填进配置即可。
- 是否需要 macOS / Linux 的 CI 矩阵。当前开发环境是 Windows，而 §7.2 的路径处理有明确的 Windows 特有风险。跨平台矩阵可以等到 `core-host-fs` 时再加——那才是第一个真正有平台差异的 change。
