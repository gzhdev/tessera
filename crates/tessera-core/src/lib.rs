//! 内核：插件生命周期编排、注册表、事件总线、权限管理、全局等待图。
//!
//! 不依赖 wasmtime（设计书 §17.1 约束 2）。本 change 只建立骨架与横切设施
//! （错误模型见 `tessera-error`，可观测性见 [`observability`]）；生命周期等实现见后续 change。

pub mod observability;
