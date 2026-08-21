//! 第 4 组：补齐组件清单的正例 fixture 与契约断言（4.1–4.6）。

use tessera_ui_schema::validate::validate_tree;
use tessera_ui_schema::{Component, UiTree};

fn parse_positive(name: &str) -> UiTree {
    let path = format!(
        "{}/tests/fixtures/positive/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path} 失败：{e}"));
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}.json 不是合法 JSON：{e}"));
    let outcome = validate_tree(&value);
    assert!(
        outcome.degraded.is_empty(),
        "{name}.json 不应产生降级：{:?}（警告：{:?}）",
        outcome.degraded,
        outcome.warnings
    );
    outcome
        .tree
        .unwrap_or_else(|| panic!("{name}.json 应解析出树，警告：{:?}", outcome.warnings))
}

macro_rules! positive_case {
    ($name:ident, $file:literal, $tag:literal) => {
        #[test]
        fn $name() {
            let tree = parse_positive($file);
            assert_eq!(
                tree.root.component.type_tag(),
                $tag,
                "{} 的 type 标记",
                $file
            );
        }
    };
}

// 4.1 布局类
positive_case!(pos_grid, "grid", "grid");
positive_case!(pos_scroll, "scroll", "scroll");
positive_case!(pos_tabs, "tabs", "tabs");
positive_case!(pos_group, "group", "group");
positive_case!(pos_spacer, "spacer", "spacer");
positive_case!(pos_split, "split", "split");
// 4.2 展示类
positive_case!(pos_heading, "heading", "heading");
positive_case!(pos_badge, "badge", "badge");
positive_case!(pos_divider, "divider", "divider");
positive_case!(pos_icon, "icon", "icon");
positive_case!(pos_markdown, "markdown", "markdown");
positive_case!(pos_code, "code", "code");
positive_case!(pos_image, "image", "image");
positive_case!(pos_empty_state, "empty-state", "empty-state");
// 4.3 输入类
positive_case!(pos_textarea, "textarea", "textarea");
positive_case!(pos_number_input, "number-input", "number-input");
positive_case!(pos_select, "select", "select");
positive_case!(pos_checkbox, "checkbox", "checkbox");
positive_case!(pos_radio_group, "radio-group", "radio-group");
positive_case!(pos_switch, "switch", "switch");
positive_case!(pos_slider, "slider", "slider");
// 4.4 数据类与反馈类
positive_case!(pos_list, "list", "list");
positive_case!(pos_tree, "tree", "tree");
positive_case!(pos_key_value, "key-value", "key-value");
positive_case!(pos_alert, "alert", "alert");
positive_case!(pos_progress, "progress", "progress");
positive_case!(pos_spinner, "spinner", "spinner");
// 4.5 file-picker
positive_case!(pos_file_picker, "file-picker", "file-picker");

/// 4.5：file-picker 在未声明任何文件系统权限的场景下契约仍完整可用。
///
/// 契约层面没有任何「权限」字段——组件是否可用与权限无关（数据交付模型），
/// 因此只要 props 契约完整解析即证明该场景成立。
#[test]
fn file_picker_usable_without_fs_permission() {
    let tree = parse_positive("file-picker");
    let root = &tree.root;
    // 组件本身无权限相关 props，事件走 pick 而非路径。
    assert_eq!(root.component.type_tag(), "file-picker");
    assert_eq!(root.on.get("pick").map(String::as_str), Some("load-csv"));
    // 载荷形状由 event.rs FilePickValue 保证（无路径字段，见 validate.rs 测试）。
}

/// 容器识别：布局类应判为容器，叶组件不应。
#[test]
fn container_classification() {
    use tessera_ui_schema::components::{GroupProps, SliderProps, TabsProps};
    let containers = [
        Component::Grid(Default::default()),
        Component::Scroll(Default::default()),
        Component::Split(Default::default()),
        Component::Group(GroupProps {
            title: "t".into(),
            collapsible: None,
        }),
        Component::Tabs(TabsProps {
            labels: vec!["a".into()],
            active: None,
        }),
    ];
    for c in &containers {
        assert!(c.is_container(), "{} 应为容器", c.type_tag());
    }
    let leaves = [
        Component::Spacer(Default::default()),
        Component::Spinner(Default::default()),
        Component::Progress(Default::default()),
        Component::Checkbox(Default::default()),
        Component::Slider(SliderProps {
            min: 0.0,
            max: 1.0,
            value: None,
            step: None,
        }),
    ];
    for c in &leaves {
        assert!(!c.is_container(), "{} 不应为容器", c.type_tag());
    }
}
