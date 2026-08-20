# Tessera

桌面插件化工具核心。内核薄、稳定、通用，业务功能全部通过插件契约挂载。
当前处于工程基座阶段（OpenSpec 变更 `core-workspace-bootstrap`）。

- 架构设计书（权威）：`plugin-core-architecture-design.md`
- 工具链版本基线：`docs/toolchain-baseline.md`
- 变更提案：`openspec/changes/`

## 环境要求

| 工具 | 版本 | 说明 |
|---|---|---|
| Rust | stable（开发基线 1.97.1） | MSVC 工具链（Windows） |
| Node.js | ≥ 20 | 前端构建 |
| npm | 随 Node | |
| wasm32-wasip2 target | — | 仅 WASM 工具链冒烟用例需要 |

Windows 构建外壳需要 WebView2 Runtime（Win11 自带）。

## 从干净 checkout 构建

```bash
# 1. 前端依赖与产物（外壳编译时按 tauri.conf.json 嵌入 ../dist）
npm install
npm run build

# 2. Rust 全仓构建与测试
cargo build --workspace
cargo test --workspace

# 3. 启动应用（开发模式；也可以 npm run tauri dev 走热重载流）
cargo run -p tessera-app
```

应用启动后会在应用数据目录写日志：

- Windows：`%APPDATA%\dev.tessera.app\logs\tessera-YYYY-MM-DD.log`（按天轮转，保留 7 天）

## 常用命令

```bash
npm run tauri dev          # 开发模式：Vite 热重载 + Tauri 窗口
cargo fmt --all            # 格式化（CI 强制 --check）
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check-deps.sh             # 依赖方向硬性检查（§17.1 约束 1/2）
bash scripts/check-log-no-network.sh   # 日志子系统无网络出口检查
```

## WASM 工具链冒烟（可选）

验证 Component Model 工具链组合（详见 `docs/toolchain-baseline.md`）：

```bash
rustup target add wasm32-wasip2
cd examples/toolchain-roundtrip
cargo test -p roundtrip-host -- --nocapture
```

用例会自动构建 guest（`wasm32-wasip2`），用 wasmtime 实例化、调用 export、
回调 import，并实测 `StoreLimits::instances` 最小可行值。

## 目录结构

```
crates/          内核 crate（manifest / ui-schema / store / sandbox / core / sdk）
                 + tessera-error（错误模型，最底层共享）
src-tauri/       Tauri 2 外壳（窗口、应用数据目录、横切设施初始化）
src/             前端 Vue3 + TS（components/ui、components/shell、types/generated、stores）
examples/        toolchain-roundtrip：工具链冒烟用例（独立 workspace）
scripts/         依赖方向与日志出口检查脚本
wit/             WIT 契约（后续 change 落位）
```
