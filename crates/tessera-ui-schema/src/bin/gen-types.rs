//! 生成 `src/types/generated/`（任务 3.6）。
//!
//! 用法：`cargo run -p tessera-ui-schema --bin gen-types`。
//! 产出 check-in 于仓库，CI 校验「重新生成后与 check-in 内容逐字节一致」——
//! 改类型未重新生成、或手改生成物，都会让 `git diff --exit-code` 失败。
//! 路径基于 `CARGO_MANIFEST_DIR`（编译期常量），与调用方 cwd 无关，保证可重复。

use std::path::{Path, PathBuf};
use ts_rs::TS;

fn main() {
    // 路径基于 CARGO_MANIFEST_DIR（编译期常量），与调用方 cwd 无关，保证可重复。
    // ts-rs 的 Config 字段私有，经 TS_RS_EXPORT_DIR 环境变量指定输出目录。
    // crates/tessera-ui-schema -> 仓库根 -> src/types/generated
    let out_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/types/generated");
    std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| panic!("创建目录 {out_dir:?} 失败：{e}"));

    // 安全：本进程独占该变量，且 export_all 是同步调用。
    unsafe {
        std::env::set_var("TS_RS_EXPORT_DIR", &out_dir);
    }
    let cfg = ts_rs::Config::from_env();

    // 单进程一次性导出：ts-rs 只在同进程内合并共享输出文件的声明。
    tessera_ui_schema::node::UiNode::export_all(&cfg)
        .unwrap_or_else(|e| panic!("导出 UiNode 失败：{e}"));
    tessera_ui_schema::tree::UiTree::export_all(&cfg)
        .unwrap_or_else(|e| panic!("导出 UiTree 失败：{e}"));
    tessera_ui_schema::event::UiEvent::export_all(&cfg)
        .unwrap_or_else(|e| panic!("导出 UiEvent 失败：{e}"));
    tessera_ui_schema::validate::Warning::export_all(&cfg)
        .unwrap_or_else(|e| panic!("导出 Warning 失败：{e}"));

    prepend_generated_banner(&out_dir);
    println!("已生成 TS 类型到 {out_dir:?}");
}

/// 给每个生成文件头部加「自动生成，勿手改」标注（ui-typegen spec 要求）。
fn prepend_generated_banner(dir: &Path) {
    let banner = "// 自动生成，勿手改。权威来源：crates/tessera-ui-schema 的 Rust 类型；\n\
                  // 重新生成：cargo run -p tessera-ui-schema --bin gen-types\n";
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("读取目录 {dir:?} 失败：{e}"));
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("读取目录项失败：{e}"));
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ts") {
            continue;
        }
        let content =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败：{e}"));
        if content.starts_with("// 自动生成，勿手改") {
            continue; // 幂等：已标注则跳过
        }
        std::fs::write(&path, format!("{banner}{content}"))
            .unwrap_or_else(|e| panic!("写入 {path:?} 失败：{e}"));
    }
}
