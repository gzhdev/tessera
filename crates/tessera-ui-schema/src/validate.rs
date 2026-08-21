//! 描述树校验与降级（design.md D3 / D4，spec「未知组件类型必须局部降级」）。
//!
//! 两段式反序列化（D4）：先用宽松中间表示 [`RawNode`] 解析整树，再逐节点
//! 尝试强类型解析为 [`UiNode`]；未知类型 / props 不符的节点标记为降级，
//! 不影响兄弟节点，不拒绝整树（D3）。
//!
//! 同时在此检出同树重复 `id` 与「props 类型不符」并记警告（spec
//! 「节点标识唯一」「属性类型不符被报告」）。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

use crate::node::UiNode;
use crate::tree::UiTree;

/// 校验产出（design.md D3）：树 + 降级清单 + 警告清单。
///
/// 不返回 `Result`——「基本可用但有节点要降级」是合法状态，由调用方
/// （宿主记日志 / 前端渲染占位符）决定如何处置降级节点。
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationOutcome {
    /// 解析出的树（根解析失败时为 `None`，见 [`Self::is_usable`]）。
    ///
    /// 未知类型节点在树中缺席（整棵降级），并列入降级清单，
    /// 渲染侧据此渲染错误占位符（§9.3）。
    pub tree: Option<UiTree>,
    /// 需要降级渲染的节点（含路径定位）。
    pub degraded: Vec<DegradedNode>,
    /// 非致命问题（重复 id、属性类型不符等）。
    pub warnings: Vec<Warning>,
}

impl ValidationOutcome {
    /// 树是否可用（根节点未整体失败）。即便有降级或警告，只要根解析出即为真。
    pub fn is_usable(&self) -> bool {
        self.tree.is_some()
    }
}

/// 一个需要降级渲染的节点。
#[derive(Debug, Clone, PartialEq)]
pub struct DegradedNode {
    /// 节点的 `id`（若可取得）。
    pub node_id: Option<String>,
    /// 未知的组件类型标记。
    pub type_tag: String,
    /// 在树中的定位路径（如 `root/children[2]/children[0]`）。
    pub path: String,
}

/// 一条校验警告（非致命）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "Warning.ts", rename_all = "camelCase")]
pub struct Warning {
    /// 警告类别。
    pub kind: WarningKind,
    /// 定位路径。
    pub path: String,
    /// 人类可读描述（中文）。
    pub message: String,
}

/// 警告类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "WarningKind.ts", rename_all = "kebab-case")]
pub enum WarningKind {
    /// 同一棵树中出现重复 `id`。
    DuplicateId,
    /// 属性类型与契约不符。
    PropTypeMismatch,
    /// 未知组件类型（伴随降级）。
    UnknownComponent,
}

/// 宽松中间表示（D4 第一阶段）：`type` 自由文本 + `props` 原始 JSON。
///
/// 与具体组件无关，不随组件增减变化（design.md Risks 第三行）。
#[derive(Debug, Clone, Deserialize)]
struct RawNode {
    #[serde(rename = "type")]
    type_tag: String,
    id: Option<String>,
    #[serde(default)]
    props: Option<serde_json::Value>,
    #[serde(default)]
    children: Vec<RawNode>,
    #[serde(default)]
    on: Option<HashMap<String, String>>,
}

/// 校验一棵 JSON 描述树，产出结构化降级结果（spec ui-schema）。
///
/// 入口接受 `serde_json::Value`（宿主已拿到的插件返回）。未知类型节点
/// 局部降级：整棵子树从渲染树中剔除并列入降级清单。
pub fn validate_tree(input: &serde_json::Value) -> ValidationOutcome {
    let mut degraded = Vec::new();
    let mut warnings = Vec::new();

    // 第一阶段：宽松解析整树。根若不是对象 / 缺 type 则整树不可用。
    // 输入形状为 `{ "root": <节点> }`（tree.rs UiTree），先取根节点。
    let root_value = match input.get("root").cloned() {
        Some(v) => v,
        None => {
            warnings.push(Warning {
                kind: WarningKind::PropTypeMismatch,
                path: "root".to_string(),
                message: "描述树缺少 `root` 字段".to_string(),
            });
            return ValidationOutcome {
                tree: None,
                degraded,
                warnings,
            };
        }
    };
    let root_raw: RawNode = match serde_json::from_value(root_value) {
        Ok(r) => r,
        Err(e) => {
            warnings.push(Warning {
                kind: WarningKind::PropTypeMismatch,
                path: "root".to_string(),
                message: format!("根节点无法按宽松形状解析：{e}"),
            });
            return ValidationOutcome {
                tree: None,
                degraded,
                warnings,
            };
        }
    };

    let mut seen_ids: HashMap<String, String> = HashMap::new();
    let root = resolve_node(
        root_raw,
        "root",
        &mut degraded,
        &mut warnings,
        &mut seen_ids,
    );

    let tree = root.map(|r| UiTree { root: r });
    ValidationOutcome {
        tree,
        degraded,
        warnings,
    }
}

/// 第二阶段：逐节点强类型解析，失败即降级（含其子树一并降级）。
fn resolve_node(
    raw: RawNode,
    path: &str,
    degraded: &mut Vec<DegradedNode>,
    warnings: &mut Vec<Warning>,
    seen_ids: &mut HashMap<String, String>,
) -> Option<UiNode> {
    // 重复 id 检出（在降级判断之前，未知节点的 id 也参与唯一性约束）。
    if let Some(id) = &raw.id {
        if let Some(first_path) = seen_ids.get(id) {
            warnings.push(Warning {
                kind: WarningKind::DuplicateId,
                path: path.to_string(),
                message: format!("重复的节点 id `{id}`（首次出现于 {first_path}）"),
            });
        } else {
            seen_ids.insert(id.clone(), path.to_string());
        }
    }

    // 构造本节点的强类型 JSON：{type,id,props,children:[],on}。
    // children 置空——子树由外层递归单独处理，避免一个坏子节点连坐本节点。
    let mut node_json = serde_json::Map::new();
    node_json.insert(
        "type".to_string(),
        serde_json::Value::String(raw.type_tag.clone()),
    );
    if let Some(id) = &raw.id {
        node_json.insert("id".to_string(), serde_json::Value::String(id.clone()));
    }
    if let Some(props) = &raw.props {
        node_json.insert("props".to_string(), props.clone());
    }
    node_json.insert("children".to_string(), serde_json::Value::Array(Vec::new()));
    if let Some(on) = &raw.on {
        node_json.insert(
            "on".to_string(),
            serde_json::to_value(on).unwrap_or(serde_json::Value::Null),
        );
    }
    let node_json = serde_json::Value::Object(node_json);

    match serde_json::from_value::<UiNode>(node_json.clone()) {
        Ok(mut node) => {
            // 本节点解析成功。递归处理子节点；任一子节点失败只降级该子树。
            let mut children = Vec::with_capacity(raw.children.len());
            for (i, child) in raw.children.into_iter().enumerate() {
                let child_path = format!("{path}/children[{i}]");
                if let Some(c) = resolve_node(child, &child_path, degraded, warnings, seen_ids) {
                    children.push(c);
                }
            }
            node.children = children;
            Some(node)
        }
        Err(e) => {
            // 区分「未知类型」与「属性类型不符」：serde 对邻接标记枚举的
            // 未知 type 报 `unknown variant`，其余（缺字段、类型错）均为 props 不符。
            let msg = e.to_string();
            let kind = if msg.contains("unknown variant") {
                WarningKind::UnknownComponent
            } else {
                WarningKind::PropTypeMismatch
            };
            warnings.push(Warning {
                kind,
                path: path.to_string(),
                message: format!(
                    "节点 `{}`（type=`{}`）解析失败：{e}",
                    raw.id.as_deref().unwrap_or("<无 id>"),
                    raw.type_tag
                ),
            });
            // 子树一并降级：整棵子树不单独渲染（spec 场景）。
            degraded.push(DegradedNode {
                node_id: raw.id.clone(),
                type_tag: raw.type_tag.clone(),
                path: path.to_string(),
            });
            None
        }
    }
}
