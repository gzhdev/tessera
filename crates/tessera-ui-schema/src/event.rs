//! 交互事件结构（设计书 §9.4）。
//!
//! 前端把用户交互打包为 [`UiEvent`] 传给 `ui.handle-ui-event`。
//! 无值交互 `value` 为空；带值交互在 `value` 中携带组件契约规定的载荷。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

use crate::components::TableRowValue;

/// UI 交互事件（设计书 §9.4）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "UiEvent.ts", rename_all = "camelCase")]
pub struct UiEvent {
    /// 触发事件的节点 `id`。
    pub node_id: String,
    /// 事件名（`click` / `change` / `rowClick` / `pick` …）。
    pub event: String,
    /// 节点 `on` 映射中声明的动作字符串（宿主原样透传）。
    pub action: String,
    /// 载荷：无值交互为空；带值交互按组件契约携带。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<EventValue>,
}

/// 事件载荷：按组件契约区分的固定形态（untagged，由结构区分）。
///
/// - 纯文本：`"..."`（text-input 等）
/// - 表格行：`{ rowIndex, row }`
/// - 文件选择：`{ token, fileName, size }`（v0.3 决策 B3，design.md D5）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "EventValue.ts")]
pub enum EventValue {
    /// 带值输入的文本载荷（text-input / textarea 等）。
    Text(String),
    /// 数字输入的载荷（number-input / slider 等）。
    Number(f64),
    /// 开关 / 勾选框的布尔载荷（switch / checkbox）。
    Bool(bool),
    /// 下拉选择的选中项（select / radio-group）。
    Selection(String),
    /// 表格行交互：`rowClick` 同时携带行序号与行数据。
    TableRow(TableRowPick),
    /// 文件选择交付：`{ token, fileName, size }`，无任何路径字段（D5）。
    FilePick(FilePickValue),
}

/// 表格行交互载荷（设计书 §9.4：`{ rowIndex, row }`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TableRowPick.ts", rename_all = "camelCase")]
pub struct TableRowPick {
    /// 行序号（0 起）。
    pub row_index: u32,
    /// 该行数据。
    pub row: BTreeMap<String, TableRowValue>,
}

/// 文件选择交付载荷（v0.3 决策 B3，design.md D5）。
///
/// `token` 是不透明字符串，插件凭它从自身私有存储的保留命名空间
/// `__picked/{token}` 读取内容（§9.2 数据交付模型）。**不含任何路径**，
/// 也不要求插件持有文件系统权限。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "FilePickValue.ts", rename_all = "camelCase")]
pub struct FilePickValue {
    /// 交付标识（不透明字符串，对应 `__picked/{token}`）。
    pub token: String,
    /// 文件名（不含目录）。
    pub file_name: String,
    /// 字节数。
    pub size: u64,
}
