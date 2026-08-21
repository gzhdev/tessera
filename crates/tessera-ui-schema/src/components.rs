//! 六个代表性组件的 props 契约（design.md 推进顺序建议：先打通管线再补齐）。
//!
//! 布局容器：[`VStackProps`] / [`HStackProps`]；纯展示：[`TextProps`]；
//! 输入（无值事件）：[`ButtonProps`]；输入（有值事件）：[`TextInputProps`]；
//! 数据（结构化载荷事件）：[`TableProps`]。
//!
//! 所有 props struct 均 `deny_unknown_fields`：契约未定义的属性在校验期
//! 被拒，配合「组件清单」Requirement 的「属性类型不符被报告」场景。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `vstack`：垂直堆叠容器（布局类代表，嵌套结构验证用）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "VStackProps.ts", rename_all = "camelCase")]
pub struct VStackProps {
    /// 子项间距（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gap: Option<u32>,
    /// 内边距（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub padding: Option<u32>,
}

/// `hstack`：水平堆叠容器。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "HStackProps.ts", rename_all = "camelCase")]
pub struct HStackProps {
    /// 子项间距（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gap: Option<u32>,
    /// 内边距（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub padding: Option<u32>,
}

/// `text`：纯展示文本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TextProps.ts", rename_all = "camelCase")]
pub struct TextProps {
    /// 文本内容。
    pub text: String,
    /// 是否弱化显示（次要信息）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub muted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub extra_probe: Option<u32>,
}

/// `button`：按钮（无值事件代表：`click` 不携带载荷）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "ButtonProps.ts", rename_all = "camelCase")]
pub struct ButtonProps {
    /// 按钮文字。
    pub label: String,
    /// 视觉变体（枚举 props 代表，验证 ts-rs 对嵌套枚举的生成）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub variant: Option<ButtonVariant>,
    /// 是否禁用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub disabled: Option<bool>,
}

/// 按钮视觉变体（设计书 §9.3 示例中出现 `primary`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "ButtonVariant.ts", rename_all = "kebab-case")]
pub enum ButtonVariant {
    Default,
    Primary,
    Danger,
}

/// `text-input`：单行文本输入（有值事件代表：`change` 携带输入文本）。
///
/// 设计书 §9.2 / design.md D6：**不设** password / masked 属性——凭据
/// 统一走配置项，宿主无法在插件 UI 树里强制风险说明。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TextInputProps.ts", rename_all = "camelCase")]
pub struct TextInputProps {
    /// 占位提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub placeholder: Option<String>,
    /// 初始值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<String>,
    /// 非受控策略下显式强制覆盖本地编辑态（§9.4 回环问题的逃生口）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub force_value: Option<bool>,
    /// 实时响应场景的节流间隔（毫秒，§9.4）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub debounce: Option<u32>,
}

/// `table`：表格（结构化载荷事件代表：`rowClick` 携带行序号与行数据）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TableProps.ts", rename_all = "camelCase")]
pub struct TableProps {
    /// 列定义。
    pub columns: Vec<TableColumn>,
    /// 行数据。
    pub rows: Vec<TableRow>,
}

/// 表格列定义（嵌套结构验证用，设计书 §9.3 示例）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TableColumn.ts", rename_all = "camelCase")]
pub struct TableColumn {
    /// 行数据中的字段名。
    pub key: String,
    /// 列标题。
    pub title: String,
    /// 列宽（如 `2fr`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub width: Option<String>,
}

/// 表格行：字段名到单元格值的映射（行数据是开放的，由插件定义列对应键）。
pub type TableRow = std::collections::BTreeMap<String, TableRowValue>;

/// 单元格值：字符串或数字（表格单元格的标量）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "TableRowValue.ts")]
pub enum TableRowValue {
    Text(String),
    Number(f64),
}

// ---------------------------------------------------------------------------
// 布局类（4.1）：grid / scroll / tabs / group / spacer / split
// ---------------------------------------------------------------------------

/// `grid`：网格容器（按列数排布子项）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "GridProps.ts", rename_all = "camelCase")]
pub struct GridProps {
    /// 列数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub columns: Option<u32>,
    /// 子项间距（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gap: Option<u32>,
}

/// `scroll`：可滚动容器。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "ScrollProps.ts", rename_all = "camelCase")]
pub struct ScrollProps {
    /// 最大高度（px），超出出现滚动条。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_height: Option<u32>,
}

/// `tabs`：标签页容器，子项按 `labels` 分页。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TabsProps.ts", rename_all = "camelCase")]
pub struct TabsProps {
    /// 各标签页标题（与子项一一对应）。
    pub labels: Vec<String>,
    /// 初始选中的标签序号（0 起）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub active: Option<u32>,
}

/// `group`：带标题的分组容器（字段集）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "GroupProps.ts", rename_all = "camelCase")]
pub struct GroupProps {
    /// 分组标题。
    pub title: String,
    /// 是否可折叠。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub collapsible: Option<bool>,
}

/// `spacer`：弹性占位（推开两侧内容）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "SpacerProps.ts", rename_all = "camelCase")]
pub struct SpacerProps {
    /// 最小尺寸（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub size: Option<u32>,
}

/// `split`：可拖拽分割的容器（两个子项）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "SplitProps.ts", rename_all = "camelCase")]
pub struct SplitProps {
    /// 方向：水平（左右）或垂直（上下）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub direction: Option<SplitDirection>,
    /// 初始分割比例（0.0–1.0，第一个子项占比）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ratio: Option<f64>,
}

/// 分割方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "SplitDirection.ts", rename_all = "kebab-case")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

// ---------------------------------------------------------------------------
// 展示类（4.2）：heading / badge / divider / icon / markdown / code / image / empty-state
// ---------------------------------------------------------------------------

/// `heading`：标题。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "HeadingProps.ts", rename_all = "camelCase")]
pub struct HeadingProps {
    /// 标题文字。
    pub text: String,
    /// 级别（1–6，设计书示例用 3）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub level: Option<u32>,
}

/// `badge`：徽标（状态小标签）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "BadgeProps.ts", rename_all = "camelCase")]
pub struct BadgeProps {
    /// 徽标文字。
    pub text: String,
    /// 色调（info / success / warning / danger）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tone: Option<BadgeTone>,
}

/// 徽标色调。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "BadgeTone.ts", rename_all = "kebab-case")]
pub enum BadgeTone {
    Info,
    Success,
    Warning,
    Danger,
}

/// `divider`：分割线。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "DividerProps.ts", rename_all = "camelCase")]
pub struct DividerProps {
    /// 方向（默认水平）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub direction: Option<SplitDirection>,
}

/// `icon`：图标。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "IconProps.ts", rename_all = "camelCase")]
pub struct IconProps {
    /// 图标名（宿主内置图标集）。
    pub name: String,
    /// 尺寸（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub size: Option<u32>,
}

/// `markdown`：Markdown 渲染。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "MarkdownProps.ts", rename_all = "camelCase")]
pub struct MarkdownProps {
    /// Markdown 源文本。
    pub source: String,
}

/// `code`：代码块。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "CodeProps.ts", rename_all = "camelCase")]
pub struct CodeProps {
    /// 代码内容。
    pub code: String,
    /// 语言（语法高亮）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub language: Option<String>,
}

/// `image`：图片（内容经数据交付，不含路径）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "ImageProps.ts", rename_all = "camelCase")]
pub struct ImageProps {
    /// 图片来源（data URL 或宿主交付的 blob 标识，不含文件系统路径）。
    pub source: String,
    /// 替代文本。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub alt: Option<String>,
    /// 宽度（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub width: Option<u32>,
}

/// `empty-state`：空态占位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "EmptyStateProps.ts", rename_all = "camelCase")]
pub struct EmptyStateProps {
    /// 主提示文字。
    pub message: String,
    /// 可选图标名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub icon: Option<String>,
}

// ---------------------------------------------------------------------------
// 输入类（4.3）：textarea / number-input / select / checkbox / radio-group / switch / slider
// ---------------------------------------------------------------------------

/// `textarea`：多行文本输入（change 携带文本）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TextareaProps.ts", rename_all = "camelCase")]
pub struct TextareaProps {
    /// 占位提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub placeholder: Option<String>,
    /// 初始值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<String>,
    /// 行数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rows: Option<u32>,
}

/// `number-input`：数字输入（change 携带数值）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "NumberInputProps.ts", rename_all = "camelCase")]
pub struct NumberInputProps {
    /// 初始值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<f64>,
    /// 最小值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub min: Option<f64>,
    /// 最大值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max: Option<f64>,
    /// 步长。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub step: Option<f64>,
}

/// `select`：下拉选择（change 携带选中项 value）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "SelectProps.ts", rename_all = "camelCase")]
pub struct SelectProps {
    /// 候选项。
    pub options: Vec<SelectOption>,
    /// 当前选中项的 value。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<String>,
    /// 占位提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub placeholder: Option<String>,
}

/// 下拉候选项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "SelectOption.ts", rename_all = "camelCase")]
pub struct SelectOption {
    /// 提交值。
    pub value: String,
    /// 显示文字。
    pub label: String,
}

/// `checkbox`：勾选框（change 携带布尔）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "CheckboxProps.ts", rename_all = "camelCase")]
pub struct CheckboxProps {
    /// 标签。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
    /// 是否勾选。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub checked: Option<bool>,
}

/// `radio-group`：单选组（change 携带选中项 value）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "RadioGroupProps.ts", rename_all = "camelCase")]
pub struct RadioGroupProps {
    /// 候选项。
    pub options: Vec<SelectOption>,
    /// 当前选中项的 value。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<String>,
}

/// `switch`：开关（change 携带布尔）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "SwitchProps.ts", rename_all = "camelCase")]
pub struct SwitchProps {
    /// 标签。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
    /// 是否开启。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub on: Option<bool>,
}

/// `slider`：滑杆（change 携带数值）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "SliderProps.ts", rename_all = "camelCase")]
pub struct SliderProps {
    /// 最小值。
    pub min: f64,
    /// 最大值。
    pub max: f64,
    /// 当前值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<f64>,
    /// 步长。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub step: Option<f64>,
}

// ---------------------------------------------------------------------------
// 数据类（4.4）：list / tree / key-value
// ---------------------------------------------------------------------------

/// `list`：列表（每项一段文本 + 可点）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "ListProps.ts", rename_all = "camelCase")]
pub struct ListProps {
    /// 列表项。
    pub items: Vec<ListItem>,
}

/// 列表项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "ListItem.ts", rename_all = "camelCase")]
pub struct ListItem {
    /// 主文字。
    pub title: String,
    /// 次要文字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub subtitle: Option<String>,
    /// 可选图标名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub icon: Option<String>,
}

/// `tree`：树形数据（节点可展开）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TreeProps.ts", rename_all = "camelCase")]
pub struct TreeProps {
    /// 根节点集合。
    pub nodes: Vec<TreeNode>,
}

/// 树节点。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "TreeNode.ts", rename_all = "camelCase")]
pub struct TreeNode {
    /// 节点文字。
    pub label: String,
    /// 子节点。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, type = "Array<TreeNode>")]
    pub children: Vec<TreeNode>,
}

/// `key-value`：键值对展示。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "KeyValueProps.ts", rename_all = "camelCase")]
pub struct KeyValueProps {
    /// 键值对。
    pub entries: Vec<KeyValueEntry>,
}

/// 键值对项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "KeyValueEntry.ts", rename_all = "camelCase")]
pub struct KeyValueEntry {
    /// 键。
    pub key: String,
    /// 值。
    pub value: String,
}

// ---------------------------------------------------------------------------
// 反馈类（4.4）：alert / progress / spinner
// ---------------------------------------------------------------------------

/// `alert`：警示条。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "AlertProps.ts", rename_all = "camelCase")]
pub struct AlertProps {
    /// 内容。
    pub message: String,
    /// 色调。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tone: Option<BadgeTone>,
}

/// `progress`：进度条。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "ProgressProps.ts", rename_all = "camelCase")]
pub struct ProgressProps {
    /// 进度（0.0–1.0；缺省为不确定态）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<f64>,
    /// 说明文字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
}

/// `spinner`：加载指示。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "SpinnerProps.ts", rename_all = "camelCase")]
pub struct SpinnerProps {
    /// 说明文字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
}

// ---------------------------------------------------------------------------
// 输入类（4.5）：file-picker（数据交付模型，v0.3 决策 B3）
// ---------------------------------------------------------------------------

/// `file-picker`：文件选择（pick 事件携带 [`crate::event::FilePickValue`]）。
///
/// v0.3 数据交付模型：插件永不持有路径、不需要 fs 权限；用户选中的文件
/// 由宿主读出内容后经保留 key `__picked/{token}` 交付，事件只带
/// `{ token, fileName, size }`。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "FilePickerProps.ts", rename_all = "camelCase")]
pub struct FilePickerProps {
    /// 按钮文字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
    /// 文件类型过滤（如 `["csv", "txt"]`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub accept: Option<Vec<String>>,
}
