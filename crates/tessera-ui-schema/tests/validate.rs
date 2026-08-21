//! 校验与降级（3.1–3.5）：未知类型局部降级、子树一并降级、重复 id、
//! 属性类型不符、on 透传、事件载荷形状。

use tessera_ui_schema::validate::{WarningKind, validate_tree};
use tessera_ui_schema::{Component, EventValue, FilePickValue, UiEvent};

fn outcome_of(value: serde_json::Value) -> tessera_ui_schema::ValidationOutcome {
    validate_tree(&value)
}

/// 3.1 / 3.2：10 个节点中 1 个未知，降级清单恰含该节点，另 9 个正常，
/// 产生警告而非错误。
#[test]
fn unknown_type_degrades_only_that_node() {
    let mut children = Vec::new();
    for i in 0..9 {
        children.push(serde_json::json!({
            "type": "text", "id": format!("ok-{i}"), "props": { "text": "x" }
        }));
    }
    children.push(serde_json::json!({ "type": "nope", "id": "bad", "props": {} }));

    let outcome = outcome_of(serde_json::json!({
        "root": { "type": "vstack", "id": "root", "props": {}, "children": children }
    }));

    assert!(outcome.is_usable(), "整树不应被拒绝");
    assert_eq!(outcome.degraded.len(), 1, "降级清单应恰含未知节点");
    assert_eq!(outcome.degraded[0].node_id.as_deref(), Some("bad"));
    assert_eq!(outcome.degraded[0].type_tag, "nope");
    let root = outcome.tree.as_ref().unwrap().root.clone();
    assert_eq!(root.children.len(), 9, "其余 9 个节点应正常保留");
    assert!(
        outcome
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::UnknownComponent),
        "应产生未知组件警告"
    );
}

/// 3.3：未知类型节点带 children 时整体降级，子树不被单独渲染。
#[test]
fn unknown_type_degrades_whole_subtree() {
    let outcome = outcome_of(serde_json::json!({
        "root": {
            "type": "vstack", "id": "root", "props": {},
            "children": [
                { "type": "nope", "id": "bad", "props": {},
                  "children": [ { "type": "text", "id": "inner", "props": { "text": "x" } } ] },
                { "type": "text", "id": "sibling", "props": { "text": "ok" } }
            ]
        }
    }));

    let root = outcome.tree.as_ref().unwrap().root.clone();
    assert_eq!(root.children.len(), 1, "未知节点整棵降级，仅剩兄弟节点");
    assert_eq!(root.children[0].id, "sibling");
    assert_eq!(outcome.degraded.len(), 1);
    assert_eq!(outcome.degraded[0].type_tag, "nope");
    // 子树中的 text 节点不应作为独立节点出现在树中（已随父降级剔除）。
    let json = serde_json::to_value(&root).unwrap();
    assert!(!json.to_string().contains("inner"));
}

/// 3.4：同树重复 id 被检出并产生可定位的警告。
#[test]
fn duplicate_id_is_warned_with_location() {
    let outcome = outcome_of(serde_json::json!({
        "root": {
            "type": "vstack", "id": "root", "props": {},
            "children": [
                { "type": "text", "id": "dup", "props": { "text": "a" } },
                { "type": "text", "id": "dup", "props": { "text": "b" } }
            ]
        }
    }));

    let dup: Vec<_> = outcome
        .warnings
        .iter()
        .filter(|w| w.kind == WarningKind::DuplicateId)
        .collect();
    assert_eq!(dup.len(), 1, "应产生一条重复 id 警告");
    assert!(dup[0].message.contains("dup"), "警告应含重复的 id 值");
    assert!(
        dup[0].path.contains("children[1]"),
        "警告应含第二次出现的位置，实际 path={}",
        dup[0].path
    );
    assert!(
        dup[0].message.contains("children[0]"),
        "警告应含首次出现位置"
    );
}

/// 3.5：数值型属性被赋字符串时，校验指出该节点该属性类型不符。
#[test]
fn prop_type_mismatch_is_reported() {
    let outcome = outcome_of(serde_json::json!({
        "root": { "type": "vstack", "id": "root", "props": { "gap": "不是数字" } }
    }));

    assert!(
        outcome
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::PropTypeMismatch),
        "应产生属性类型不符警告"
    );
    assert_eq!(outcome.degraded.len(), 1, "类型不符的节点进入降级清单");
    assert!(outcome.tree.is_none(), "根节点类型不符则整树不可用");
}

/// 2.3：on 动作字符串不被解释或校验，任意自定义动作名解析成功并原样保留。
#[test]
fn on_action_strings_are_passed_through() {
    let outcome = outcome_of(serde_json::json!({
        "root": { "type": "button", "id": "b", "props": { "label": "x" },
                  "on": { "click": "任意自定义动作名-!@#$%", "hover": "另一个" } }
    }));
    let root = outcome.tree.unwrap().root;
    assert_eq!(
        root.on.get("click").map(String::as_str),
        Some("任意自定义动作名-!@#$%")
    );
    assert_eq!(root.on.get("hover").map(String::as_str), Some("另一个"));
}

/// 2.4：交互事件结构——无值 value 为空；带值按组件契约携带；表格行携带行序号与行数据。
#[test]
fn ui_event_shapes() {
    // 无值：按钮点击
    let click: UiEvent = serde_json::from_value(serde_json::json!({
        "nodeId": "sync", "event": "click", "action": "do-sync"
    }))
    .expect("无值事件解析");
    assert!(click.value.is_none());

    // 带值：文本输入提交
    let change: UiEvent = serde_json::from_value(serde_json::json!({
        "nodeId": "endpoint", "event": "change", "action": "set-endpoint",
        "value": "https://api.example.com"
    }))
    .expect("文本事件解析");
    assert!(
        matches!(change.value, Some(EventValue::Text(ref s)) if s == "https://api.example.com")
    );

    // 表格行交互：行序号 + 行数据
    let row: UiEvent = serde_json::from_value(serde_json::json!({
        "nodeId": "files", "event": "rowClick", "action": "open-file",
        "value": { "rowIndex": 1, "row": { "name": "b.txt", "status": "待处理" } }
    }))
    .expect("表格行事件解析");
    match row.value {
        Some(EventValue::TableRow(p)) => {
            assert_eq!(p.row_index, 1);
            assert!(p.row.contains_key("name"));
        }
        other => panic!("表格行载荷形状错误：{other:?}"),
    }
}

/// 4.5（在此一并验证事件侧）：file-picker 载荷为 { token, fileName, size }，无路径字段。
#[test]
fn file_pick_value_has_no_path() {
    let v = FilePickValue {
        token: "a1b2c3d4".into(),
        file_name: "data.csv".into(),
        size: 1_048_576,
    };
    let json = serde_json::to_value(&v).unwrap();
    let obj = json.as_object().unwrap();
    assert!(obj.contains_key("token") && obj.contains_key("fileName") && obj.contains_key("size"));
    for k in obj.keys() {
        assert!(
            !k.to_lowercase().contains("path") && !k.to_lowercase().contains("dir"),
            "载荷不应含路径字段，发现 {k}"
        );
    }
    // 反序列化带路径字段的载荷会因 deny_unknown_fields 失败。
    assert!(
        serde_json::from_value::<FilePickValue>(serde_json::json!({
            "token": "t", "fileName": "f", "size": 1, "path": "C:/etc/passwd"
        }))
        .is_err()
    );
}

/// 4.6（类型层证据）：TextInputProps 不存在 password / masked 字段，
/// 为文本输入声明掩码属性会因 deny_unknown_fields 校验失败。
#[test]
fn text_input_has_no_password_field() {
    let outcome = outcome_of(serde_json::json!({
        "root": { "type": "text-input", "id": "pw",
                  "props": { "placeholder": "x", "password": true } }
    }));
    assert!(
        outcome
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::PropTypeMismatch),
        "声明 password 应导致类型不符"
    );
    assert_eq!(outcome.degraded.len(), 1);
}

/// 组件枚举的 type 标记与设计书 §9.3 示例一致。
#[test]
fn component_type_tags_match_design_doc() {
    assert_eq!(Component::Button(Default::default()).type_tag(), "button");
    assert_eq!(
        Component::TextInput(Default::default()).type_tag(),
        "text-input"
    );
    assert_eq!(Component::VStack(Default::default()).type_tag(), "vstack");
}
