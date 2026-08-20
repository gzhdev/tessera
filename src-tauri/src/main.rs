//! Tessera 桌面外壳：窗口创建、应用数据目录解析、横切设施初始化。
//!
//! 本 crate 只是把内核接到窗口与 IPC 上的胶水层（设计书 §17.1 约束 1）；
//! 业务逻辑一律不下沉到这里。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    use tauri::Manager;

    tauri::Builder::default()
        .setup(|app| {
            // 应用数据目录：由 identifier（dev.tessera.app）决定
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("解析应用数据目录失败：{e}"))?;
            std::fs::create_dir_all(&data_dir).map_err(|e| format!("创建应用数据目录失败：{e}"))?;

            // 初始化横切设施（任务 6.1）：日志先行，便于记录后续启动问题
            tessera_core::observability::init_global(&data_dir.join("logs"))
                .map_err(|e| format!("初始化日志系统失败：{e}"))?;
            tracing::info!(
                target: "tessera::boot",
                data_dir = %data_dir.display(),
                "Tessera 外壳启动完成"
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tessera 启动失败");
}
