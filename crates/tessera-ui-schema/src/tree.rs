//! 描述树的顶层包装：一个视图一棵根节点树（§9.3）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::node::UiNode;

/// 一棵 UI 描述树（一个视图对应一棵根节点）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "UiTree.ts", rename_all = "camelCase")]
pub struct UiTree {
    /// 根节点。
    pub root: UiNode,
}
