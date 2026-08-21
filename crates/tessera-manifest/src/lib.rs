//! 插件清单：类型定义（[`manifest`]）、解析校验流水线（[`validate`]）、
//! 依赖图算法（[`depgraph`]）、JSON Schema 生成（[`schema`]）。
//!
//! 本 crate 是清单校验的真值所在（设计书 §4.3）：`deny_unknown_fields` 的
//! serde 类型拒绝未知字段，`schema` 模块生成的 `plugin.schema.json` 只是
//! 供编辑器补全的**生成物**，不是校验入口。
//!
//! 依赖方向（§17.1）：不依赖 `tessera-core`，跨插件的冲突检查编排归
//! `core-plugin-lifecycle`——本 crate 只提供纯函数与依赖图算法。

pub mod depgraph;
pub mod ids;
pub mod manifest;
pub mod schema;
pub mod validate;

pub use depgraph::{DepGraph, DepNode, DepResolution};
pub use ids::{LocalName, PluginId, QualifiedId};
pub use manifest::{
    ActivationEvent, CommandContribution, ConfigType, Configuration, ConfigurationProperty,
    Contributes, Engines, Keybinding, MenuEntry, Permission, PluginManifest, SemverRange,
    StrictVersion, ViewContribution, ViewKind,
};
pub use validate::{HostCompat, ValidatedManifest, ValidationContext, ValidationWarning};
