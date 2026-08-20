# 工具链基线（toolchain-baseline）

> 本文是 `core-workspace-bootstrap` 变更第 1 组任务的产物，记录一组**经过实测验证**的
> WASM Component Model 工具链版本组合与构建命令。`examples/toolchain-roundtrip/` 中的
> 冒烟用例常驻 CI，任何工具链升级都应先跑通它（见 `specs/build-constraints/spec.md`
> 「工具链版本组合必须被锁定且可复现」）。

## 验证环境

- Windows 11（10.0.26200）x64，MSVC 工具链
- 宿主 crate 源：本机 cargo 镜像（不影响版本解析结果）

## 版本组合（2026-08-20 实测通过）

| 环节 | 工具 / crate | 版本 | 验证命令 |
|---|---|---|---|
| Rust 编译器 | rustc（stable-x86_64-pc-windows-msvc） | 1.97.1 | `rustc --version` |
| 包管理 | cargo | 1.97.1 | `cargo --version` |
| WASM 目标 | rustup target | `wasm32-wasip2` | `rustup target list --installed` |
| guest 绑定生成 | wit-bindgen（crate，宏方式） | 0.60.0 | `cargo build -p roundtrip-guest --target wasm32-wasip2` |
| guest 绑定生成（CLI） | wit-bindgen-cli | 0.60.0 | `wit-bindgen --version` |
| component 检视/封装 | wasm-tools | 1.257.1 | `wasm-tools --version` |
| 宿主运行时 | wasmtime（crate，feature `component-model`） | 47.0.3 | 见往返用例 |
| 宿主 WASI 实现 | wasmtime-wasi（crate，`p2::add_to_linker_sync`） | 47.0.3 | 见往返用例 |
| 组件内 WASI 接口版本 | 由 std 带入 | wasi 0.2.9 系列 | `wasm-tools component wit` 反解可见 |

安装/锁定命令：

```bash
rustup toolchain install stable
rustup target add wasm32-wasip2
cargo install wit-bindgen-cli --version 0.60.0 --locked
cargo install wasm-tools --version 1.257.1 --locked
```

wasmtime / wasmtime-wasi / wit-bindgen（crate）的版本由 `examples/toolchain-roundtrip/*/Cargo.toml`
锁定，升级时改版本号后必须重跑冒烟用例。

## 从零重跑往返用例（1.3–1.4 的可复现步骤）

```bash
cd examples/toolchain-roundtrip

# 1.3 guest 侧：编译为 component（产物直出 component，见下方「与任务描述的差异」）
cargo build -p roundtrip-guest --target wasm32-wasip2
wasm-tools component wit target/wasm32-wasip2/debug/roundtrip_guest.wasm
#   → 反解结果应包含：import host-tag: func(name: string) -> string;
#                     export plugin-run: func(input: string) -> string;
#     （另含 std 带入的 wasi:cli / wasi:io 等 0.2.9 接口，属正常现象）

# 1.4 host 侧：实例化 + 调用 export + 回调 import（未构建 guest 时测试会自动先构建）
cargo test -p roundtrip-host -- --nocapture
#   → roundtrip_calls_back_into_host ... ok
#   → store_limits_instances_probe ... ok（stderr 打印 instances 最小可行值）
```

## StoreLimits::instances 实测结论（1.5，供 core-wasm-sandbox 采用）

**设计书 §8.2 把 `StoreLimits::instances` 定为 1——实测不够用。**

对一个由 Rust 1.97.1 + wit-bindgen 0.60.0 产出的 `wasm32-wasip2` reactor component
（guest 内含 std 与 WASI shim），从 1 开始逐档试探，结果：

- `instances(1)`：实例化失败
- `instances(2)`：实例化失败
- **`instances(3)`：实例化 + 调用 + 宿主回调全链路通过（最小可行值）**

原因：component 实例化会创建不止一个 core instance（主模块 + std/WASI 相关实例）。
**`core-wasm-sandbox` 实施时不得沿用 1**，建议：

- 默认值取 `8`（3 的实际需求 + 余量，覆盖未来 guest 结构变化）；
- 或在加载时按 component 实测（探针逻辑见 `host/tests/roundtrip.rs` 的
  `store_limits_instances_probe`）。

## 与任务描述/设计书的偏差（实测发现）

1. **`wasm-tools component new` 不再需要**。`wasm32-wasip2` 目标默认经
   wasm-component-ld 链接，`cargo build` 产物**已经是 component**（对它执行
   `component new` 会报 "decoding a component is not supported"）。该命令属于
   旧 `wasm32-wasip1` 流程。
2. **wit-bindgen CLI 没有 host 生成器**（子命令仅 rust/c/go/csharp/… guest 方向）。
   宿主侧绑定由 `wasmtime::component::bindgen!` 宏在编译期从同一份 WIT 生成，
   「双向」= guest 用 wit-bindgen + host 用 bindgen! 宏，两方向共用 `wit/roundtrip.wit`。
3. **`Config::async_support(false)` 在 wasmtime 47 已废弃**（no longer has any effect）。
   同步是默认行为；异步才需要显式开启（`wasm_component_model_async`）。
   设计书 §8.1 的该行配置在实现时应删除。
4. **bindgen! 生成的 `add_to_linker` 需要显式 `HasSelf` 标注**（wasmtime 47 引入
   `HasData` 抽象）：`Roundtrip::add_to_linker::<T, HasSelf<T>>(&mut linker, |s| s)`。
5. **`wasm-tools component wit` 反解会丢失 package/world 命名**（显示为
   `root:component/root`），但函数签名逐一致。冒烟校验以接口签名为准，不比对命名。
6. **guest 会传递依赖 WASI 0.2.9 接口**（std 带入），宿主实例化时必须链接
   `wasmtime-wasi`（`p2::add_to_linker_sync`），否则 import 无法满足。
