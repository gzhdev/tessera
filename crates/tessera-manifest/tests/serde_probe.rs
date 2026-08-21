//! serde 行为探针（design.md D3 的「必须首日验证」，任务 1.1/1.2 的结论记录）。
//!
//! 问题：internally tagged enum（`#[serde(tag = "type")]`）在分发到 variant 之前会
//! 把内容缓冲进中间表示，`deny_unknown_fields` 直接标在 enum 上是否生效？
//!
//! **2026-08 实测结论（serde 1.0.2xx）：生效。**
//! `{"type": "fs.read", "virtualDirectory": "workspace"}` 反序列化失败，
//! 错误 `unknown field \`virtualDirectory\`, expected \`virtualDir\``。
//! 设计书担心的历史行为不一致在当前版本不存在，无需启用退路。
//!
//! - 探针 A 断言直接标注形态生效——它是正式类型（`src/manifest.rs`）的形态依据；
//!   若未来 serde 升级后此测试失败（属性重新被静默忽略），按 design.md D3 启用退路。
//! - 探针 B 记录退路 1（variant 载荷独立 struct）同样有效，作为已验证的备选。

use serde::Deserialize;

/// 探针 A：`deny_unknown_fields` 直接标在 internally tagged enum 上——实测生效。
#[test]
fn probe_a_enum_direct_deny_rejects_unknown_field() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    #[serde(tag = "type", deny_unknown_fields, rename_all_fields = "camelCase")]
    enum DirectPermission {
        #[serde(rename = "fs.read")]
        FsRead { virtual_dir: String },
        #[serde(rename = "fs.write")]
        FsWrite { virtual_dir: String },
    }

    // 未知字段 virtualDirectory（正确字段名是 virtualDir）
    let err = serde_json::from_str::<DirectPermission>(
        r#"{"type": "fs.read", "virtualDirectory": "workspace"}"#,
    )
    .expect_err("enum 直接标注 deny_unknown_fields 必须拒绝未知字段");

    let msg = err.to_string();
    assert!(
        msg.contains("virtualDirectory"),
        "错误信息必须指出未知字段本身，实际：{msg}"
    );

    // 对照：正确字段名解析成功
    let ok: DirectPermission =
        serde_json::from_str(r#"{"type": "fs.read", "virtualDir": "workspace"}"#)
            .expect("正确字段名必须解析成功");
    match ok {
        DirectPermission::FsRead { virtual_dir } => assert_eq!(virtual_dir, "workspace"),
        other => panic!("分发到了错误的 variant：{other:?}"),
    }
}

/// 探针 B：退路 1——variant 载荷抽成独立 struct 并单独标注
/// `deny_unknown_fields`，enum 以 newtype variant 引用。
///
/// enum 本身不再标注 deny（只负责按 `type` 分发）；未知字段的
/// 拒绝发生在载荷 struct 自己的 Deserialize 里，行为可靠。
#[test]
fn probe_b_per_variant_struct_deny_rejects_unknown_field() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    struct FsScope {
        virtual_dir: String,
    }

    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    #[serde(tag = "type", rename_all = "camelCase")]
    enum ScopedPermission {
        #[serde(rename = "fs.read")]
        FsRead(FsScope),
        #[serde(rename = "fs.write")]
        FsWrite(FsScope),
    }

    let err = serde_json::from_str::<ScopedPermission>(
        r#"{"type": "fs.read", "virtualDirectory": "workspace"}"#,
    )
    .expect_err("载荷 struct 的 deny_unknown_fields 必须拒绝未知字段");

    let msg = err.to_string();
    assert!(
        msg.contains("virtualDirectory"),
        "错误信息必须指出未知字段本身，实际：{msg}"
    );
}
