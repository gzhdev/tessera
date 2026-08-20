# core-workspace-bootstrap · 任务

> 顺序不是建议：第 1 组必须最先完成。它的结论会影响第 2 组的 crate 依赖与后续所有 change 的可行性（design.md D1、Risks 第一行）。

## 1. 工具链基线验证

- [x] 1.1 安装并锁定工具链：Rust toolchain、`wasm32-wasip2` target、`wit-bindgen`、`wasm-tools`、`wasmtime`。验证：每个工具的 `--version` 都能输出，版本号记录待用
- [x] 1.2 在 `examples/toolchain-roundtrip/` 写一份最小 WIT：1 个 import（宿主提供的函数）+ 1 个 export（插件提供的函数）。验证：`wit-bindgen` 对该 WIT 双向生成绑定成功
- [x] 1.3 实现 guest 侧并编译为 component：`cargo build --target wasm32-wasip2` + `wasm-tools component new`。验证：产出的 `.wasm` 经 `wasm-tools component wit` 能反解出与源 WIT 一致的接口
- [x] 1.4 实现 host 侧：wasmtime 实例化该 component、调用其 export、并在 export 内部回调 host 的 import。验证：`cargo test` 中该往返用例通过，host 函数确实被调用到
- [x] 1.5 实测 `StoreLimits::instances` 对该 component 的最小可行值。验证：记录下实际需要的值——设计书 §8.2 定的 1 若不够用，在基线文档中标注正确值供 `core-wasm-sandbox` 采用
- [x] 1.6 产出 `docs/toolchain-baseline.md`：可复现的版本组合、构建命令、以及 1.5 的实测结论。验证：按文档在一台未配置过的机器（或清空 cargo 缓存后）能重跑通 1.3–1.4

## 2. Workspace 与 crate 骨架

- [x] 2.1 把根 `Cargo.toml` 改为 `[workspace]`，把现有 `src/main.rs` 移入 `src-tauri/src/`。验证：`cargo build` 通过
- [x] 2.2 创建 5 个内核 crate 骨架：`crates/tessera-manifest`、`tessera-ui-schema`、`tessera-store`、`tessera-sandbox`、`tessera-core`。验证：`cargo build --workspace` 通过
- [x] 2.3 创建 `crates/tessera-sdk` 骨架（`crate-type = ["cdylib"]`，target `wasm32-wasip2`）。验证：`cargo build -p tessera-sdk --target wasm32-wasip2` 通过
- [x] 2.4 按设计书 §17.1 的方向图连接 crate 依赖边（`sandbox → manifest, ui-schema`；`core → manifest, store, sandbox`；`src-tauri → core`）。验证：`cargo tree` 输出的依赖方向与 §17.1 图一致，无反向边

## 3. 依赖方向的 CI 强制

> 对应 `specs/build-constraints/spec.md` 的前两条 Requirement

- [x] 3.1 写检查脚本：对 5 个内核 crate 各跑一次 `cargo tree -i tauri -p <crate> --all-features`，任一有输出即失败并打印引入路径。验证：手动在 `tessera-core` 加一条 tauri 依赖，脚本报错并指出路径；移除后脚本通过
- [x] 3.2 写检查脚本：对 `tessera-core` 跑 `cargo tree -i wasmtime -p tessera-core --all-features`，有输出即失败。验证：手动加依赖后脚本报错，移除后通过
- [ ] 3.3 配置 CI 流水线：fmt、clippy（`-D warnings`）、`cargo test --workspace`、3.1、3.2，以及第 1 组的往返冒烟用例。验证：CI 在一次提交上全绿；故意提交一次违规依赖后 CI 变红

## 4. 错误模型

> 对应 `specs/error-model/spec.md`

- [x] 4.1 定义 `ErrorCode` enum，覆盖设计书 §12.1 的全部 12 个域及其已知成员，按域分模块组织。验证：单元测试断言每个错误码的字符串形式符合 `E_{DOMAIN}_{REASON}` 格式
- [x] 4.2 实现 `ErrorCode` 的字符串序列化与反序列化。验证：对全部错误码做 round-trip 测试，反序列化结果与原值相等
- [x] 4.3 定义三段式错误结构（错误码 / 面向用户的一句话 / 可选的开发者细节），并提供构造辅助。验证：单元测试覆盖「有开发者细节」与「无开发者细节」两种构造均合法
- [x] 4.4 为 `E_HOST_*` 域提供专用构造入口，构造时自动打上宿主 bug 标记。验证：单元测试断言经该入口构造的错误带有该标记，其他域的错误不带

## 5. 可观测性基座

> 对应 `specs/observability/spec.md`

- [x] 5.1 初始化 `tracing` + `tracing-subscriber`，输出到应用数据目录下的日志文件，按天轮转、保留 7 天。验证：模拟跨天写入产生两个文件；放入一个 8 天前的文件后轮转将其删除
- [x] 5.2 定义插件日志的 span 约定（强制携带插件标识）与来源标记（区分宿主写入与插件写入）。验证：写入混合日志后，能仅凭插件标识筛出该插件的全部条目，且两种来源可区分
- [x] 5.3 实现限速与截断 Layer：单条 8 KB 截断、单插件每秒 100 条限速、进入丢弃状态时记一条警告。验证：单元测试断言 1 MB 的条目被截断且标明截断；一秒内 10000 条时写入不超过 100 条并产生警告
- [x] 5.4 验证限速按插件独立：插件 A 触发限速时，插件 B 与宿主自身的日志不受影响。验证：并发写入测试断言 B 与宿主的条目数未被削减
- [x] 5.5 确认日志无网络出口。验证：代码检查加测试断言——日志子系统的依赖树中不含任何 HTTP/网络客户端

## 6. 外壳与前端脚手架

- [x] 6.1 配置 `src-tauri`：窗口创建、应用数据目录解析、启动时初始化第 4/5 组的错误与日志设施。验证：应用能启动并在日志文件中留下启动记录
- [x] 6.2 手搭前端脚手架（Vite + Vue 3 + TypeScript），按设计书 §17 建立 `components/ui`、`components/shell`、`types/generated`、`stores` 四个目录。验证：`tauri dev` 能起窗口并渲染占位页
- [x] 6.3 端到端确认：从干净 checkout 出发，按 README 的步骤能完成构建并启动应用。验证：在清空构建缓存后完整走一遍，无需额外的口头说明即可跑通
