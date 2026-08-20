//! 插件侧 Rust SDK，编译目标 `wasm32-wasip2`。
//!
//! 只依赖 `wit/` 契约，不依赖任何宿主 crate（设计书 §17.1 约束 3）。
//! 本 change 只建立骨架；实现见 `sdk-rust`（设计书 §14.2）。
