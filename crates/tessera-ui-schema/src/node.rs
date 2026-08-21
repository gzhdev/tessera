//! 节点结构与组件标记枚举（design.md D2）。
//!
//! 组件类型与 props 用同一个邻接标记枚举表达：
//! `#[serde(tag = "type", content = "props")]`，JSON 形状即
//! `{ "type": "button", "id": "ok", "props": { ... }, ... }`（设计书 §9.3）。
//!
//! 每个组件一个 props struct（`deny_unknown_fields`：传入契约未定义的属性
//! 会被拒绝）。组件清单共 34 个，覆盖设计书 §9.2 的五类（布局 / 展示 /
//! 输入 / 数据 / 反馈），定义见 `components.rs`。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

use crate::components::{
    AlertProps, BadgeProps, ButtonProps, CheckboxProps, CodeProps, DividerProps, EmptyStateProps,
    FilePickerProps, GridProps, GroupProps, HStackProps, HeadingProps, IconProps, ImageProps,
    KeyValueProps, ListProps, MarkdownProps, NumberInputProps, ProgressProps, RadioGroupProps,
    ScrollProps, SelectProps, SliderProps, SpacerProps, SpinnerProps, SplitProps, SwitchProps,
    TableProps, TabsProps, TextInputProps, TextProps, TextareaProps, TreeProps, VStackProps,
};

/// UI 描述树的一个节点（设计书 §9.3 字段约定）。
///
/// `type` 与 `id` 必须存在；`props` 由组件枚举承载；`children` 仅容器类
/// 组件有；`on` 是事件名到动作字符串的映射，动作字符串宿主原样透传。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "UiNode.ts", rename_all = "camelCase")]
pub struct UiNode {
    /// 视图内唯一标识，跨重绘稳定（diff key、焦点与滚动位置保持的依据）。
    pub id: String,
    /// 组件类型与属性（邻接标记枚举：`type` + `props`）。
    #[serde(flatten)]
    #[ts(flatten)]
    pub component: Component,
    /// 子节点，仅容器类组件有；缺省视为空。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, type = "Array<UiNode>")]
    pub children: Vec<UiNode>,
    /// 事件名 → 动作字符串。动作由插件自定义，宿主不解释。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[ts(optional, type = "Record<string, string>")]
    pub on: BTreeMap<String, String>,
}

/// 组件标记枚举（design.md D2）。
///
/// 未知组件类型不在此枚举中——它会在两段式反序列化（validate 模块）的
/// 第二阶段被检出并局部降级，而不是让整棵树反序列化失败（设计书 §9.3、
/// design.md D4）。
///
/// 标记字符串经逐变体 `rename` 显式指定（`vstack`/`hstack`/`text-input`），
/// 不依赖大小写转换规则——`rename_all = "kebab-case"` 会把 `VStack` 误转为
/// `v-stack`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "props")]
#[ts(export_to = "Component.ts")]
pub enum Component {
    #[serde(rename = "vstack")]
    #[ts(rename = "vstack")]
    VStack(VStackProps),
    #[serde(rename = "hstack")]
    #[ts(rename = "hstack")]
    HStack(HStackProps),
    #[serde(rename = "text")]
    #[ts(rename = "text")]
    Text(TextProps),
    #[serde(rename = "button")]
    #[ts(rename = "button")]
    Button(ButtonProps),
    #[serde(rename = "text-input")]
    #[ts(rename = "text-input")]
    TextInput(TextInputProps),
    #[serde(rename = "table")]
    #[ts(rename = "table")]
    Table(TableProps),
    // 布局类（4.1）
    #[serde(rename = "grid")]
    #[ts(rename = "grid")]
    Grid(GridProps),
    #[serde(rename = "scroll")]
    #[ts(rename = "scroll")]
    Scroll(ScrollProps),
    #[serde(rename = "tabs")]
    #[ts(rename = "tabs")]
    Tabs(TabsProps),
    #[serde(rename = "group")]
    #[ts(rename = "group")]
    Group(GroupProps),
    #[serde(rename = "spacer")]
    #[ts(rename = "spacer")]
    Spacer(SpacerProps),
    #[serde(rename = "split")]
    #[ts(rename = "split")]
    Split(SplitProps),
    // 展示类（4.2）
    #[serde(rename = "heading")]
    #[ts(rename = "heading")]
    Heading(HeadingProps),
    #[serde(rename = "badge")]
    #[ts(rename = "badge")]
    Badge(BadgeProps),
    #[serde(rename = "divider")]
    #[ts(rename = "divider")]
    Divider(DividerProps),
    #[serde(rename = "icon")]
    #[ts(rename = "icon")]
    Icon(IconProps),
    #[serde(rename = "markdown")]
    #[ts(rename = "markdown")]
    Markdown(MarkdownProps),
    #[serde(rename = "code")]
    #[ts(rename = "code")]
    Code(CodeProps),
    #[serde(rename = "image")]
    #[ts(rename = "image")]
    Image(ImageProps),
    #[serde(rename = "empty-state")]
    #[ts(rename = "empty-state")]
    EmptyState(EmptyStateProps),
    // 输入类（4.3 / 4.5）
    #[serde(rename = "textarea")]
    #[ts(rename = "textarea")]
    Textarea(TextareaProps),
    #[serde(rename = "number-input")]
    #[ts(rename = "number-input")]
    NumberInput(NumberInputProps),
    #[serde(rename = "select")]
    #[ts(rename = "select")]
    Select(SelectProps),
    #[serde(rename = "checkbox")]
    #[ts(rename = "checkbox")]
    Checkbox(CheckboxProps),
    #[serde(rename = "radio-group")]
    #[ts(rename = "radio-group")]
    RadioGroup(RadioGroupProps),
    #[serde(rename = "switch")]
    #[ts(rename = "switch")]
    Switch(SwitchProps),
    #[serde(rename = "slider")]
    #[ts(rename = "slider")]
    Slider(SliderProps),
    #[serde(rename = "file-picker")]
    #[ts(rename = "file-picker")]
    FilePicker(FilePickerProps),
    // 数据类（4.4）
    #[serde(rename = "list")]
    #[ts(rename = "list")]
    List(ListProps),
    #[serde(rename = "tree")]
    #[ts(rename = "tree")]
    Tree(TreeProps),
    #[serde(rename = "key-value")]
    #[ts(rename = "key-value")]
    KeyValue(KeyValueProps),
    // 反馈类（4.4）
    #[serde(rename = "alert")]
    #[ts(rename = "alert")]
    Alert(AlertProps),
    #[serde(rename = "progress")]
    #[ts(rename = "progress")]
    Progress(ProgressProps),
    #[serde(rename = "spinner")]
    #[ts(rename = "spinner")]
    Spinner(SpinnerProps),
}

/// 组件标记清单——`type_tag()` 与 [`is_known_type`] 的单一事实源。
///
/// 一处宏同时生成两者，编译期保证同步；新增组件在此追加一行即可
/// （顺序与枚举变体一致，见 ADDING_A_COMPONENT.md）。
macro_rules! component_tags {
    ($($variant:ident => $tag:literal),* $(,)?) => {
        impl Component {
            /// 组件类型标记（JSON 中的 `type` 字段值）。
            pub fn type_tag(&self) -> &'static str {
                match self {
                    $(Component::$variant(_) => $tag,)*
                }
            }

            /// 类型标记是否在组件清单内（validate 模块的白名单判别：
            /// 区分「未知组件」与「属性类型不符」）。
            pub fn is_known_type(tag: &str) -> bool {
                matches!(tag, $($tag)|*)
            }
        }
    };
}

component_tags! {
    VStack => "vstack",
    HStack => "hstack",
    Text => "text",
    Button => "button",
    TextInput => "text-input",
    Table => "table",
    Grid => "grid",
    Scroll => "scroll",
    Tabs => "tabs",
    Group => "group",
    Spacer => "spacer",
    Split => "split",
    Heading => "heading",
    Badge => "badge",
    Divider => "divider",
    Icon => "icon",
    Markdown => "markdown",
    Code => "code",
    Image => "image",
    EmptyState => "empty-state",
    Textarea => "textarea",
    NumberInput => "number-input",
    Select => "select",
    Checkbox => "checkbox",
    RadioGroup => "radio-group",
    Switch => "switch",
    Slider => "slider",
    FilePicker => "file-picker",
    List => "list",
    Tree => "tree",
    KeyValue => "key-value",
    Alert => "alert",
    Progress => "progress",
    Spinner => "spinner",
}

impl Component {
    /// 该组件是否为容器类（`children` 有意义）。
    pub fn is_container(&self) -> bool {
        matches!(
            self,
            Component::VStack(_)
                | Component::HStack(_)
                | Component::Grid(_)
                | Component::Scroll(_)
                | Component::Tabs(_)
                | Component::Group(_)
                | Component::Split(_)
        )
    }
}
