//! UI 描述树的类型定义与校验，是前端 TS 类型的生成源（ts-rs）。
//!
//! 契约权威源（设计书 §17.1 约束 5）：前端 `src/types/generated/` 全部由
//! 本 crate 的 Rust 类型经 `cargo run -p tessera-ui-schema --bin gen-types`
//! 生成，不手写、不手改。
//!
//! 本 change（ui-schema-and-typegen）只定义契约与生成管线，不实现任何
//! `.vue` 组件（属 `ui-component-library`），也不接插件（属 `core-ui-bridge`）。

pub mod components;
pub mod event;
pub mod node;
pub mod tree;
pub mod validate;

pub use event::{EventValue, FilePickValue, TableRowPick, UiEvent};
pub use node::{Component, UiNode};
pub use tree::UiTree;
pub use validate::{DegradedNode, ValidationOutcome, Warning, WarningKind};
