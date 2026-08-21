//! 生成 `schema/plugin.schema.json`（任务 6.1）。
//!
//! 用法：`cargo run -p tessera-manifest --bin gen-schema`。
//! 产出 check-in 于仓库，CI 校验「重新生成后与 check-in 内容逐字节一致」——
//! 改类型未重新生成、或手改生成物，都会让 `git diff --exit-code` 失败。
//! 路径基于 `CARGO_MANIFEST_DIR`（编译期常量），与调用方 cwd 无关，保证可重复。

fn main() {
    let out = format!("{}/schema/plugin.schema.json", env!("CARGO_MANIFEST_DIR"));
    if let Some(parent) = std::path::Path::new(&out).parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("创建目录 {parent:?} 失败：{e}"));
    }
    let content = tessera_manifest::schema::generate_schema_json();
    std::fs::write(&out, content).unwrap_or_else(|e| panic!("写入 {out} 失败：{e}"));
    println!("已生成 {out}");
}
