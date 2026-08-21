//! 任务 5.1–5.2 验证：从无文件开始的完整生命周期；跨重启数据完整、迁移不重复执行。

mod common;

use tessera_store::{config, migrate, permissions, plugins, settings, storage, vdirs};

/// 5.1：建库 → 迁移 → 内置别名 → 登记插件 → 写读配置与私有数据 → 卸载，无残留。
#[test]
fn full_lifecycle_from_scratch() {
    let (dir, path) = common::temp_db();
    assert!(!path.exists(), "起点必须是无数据库文件");
    let pool = tessera_store::open(&path).expect("从零启动建库");
    assert!(path.exists(), "数据库文件应被创建");

    // 迁移已应用、内置别名就位
    let aliases = vdirs::list(&pool).expect("列别名");
    assert_eq!(aliases.len(), 3);
    assert!(aliases.iter().all(|a| a.builtin));

    // 登记插件并写读各类数据
    common::register_plugin(&pool, "com.example.e2e");
    permissions::grant(
        &pool,
        "com.example.e2e",
        "fs.read",
        "{\"virtualDir\":\"workspace\"}",
    )
    .expect("授权");
    config::set_value(&pool, "com.example.e2e", "theme", "\"dark\"").expect("写配置");
    assert_eq!(
        config::get_value(&pool, "com.example.e2e", "theme").expect("读配置"),
        Some("\"dark\"".to_string())
    );
    storage::set_plugin_key(&pool, "com.example.e2e", "state/index", &[1, 2, 3])
        .expect("写私有数据");
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.e2e", "state/index").expect("读私有数据"),
        Some(vec![1, 2, 3])
    );
    storage::set_reserved_key(&pool, "com.example.e2e", "__picked/tok", b"file-bytes")
        .expect("宿主交付");
    settings::set(&pool, "window.layout", "{\"w\":1024,\"h\":768}").expect("写设置");

    // 卸载后无任何残留行（全部表直查）
    plugins::uninstall(&pool, "com.example.e2e").expect("卸载");
    let conn = pool.get().expect("取连接");
    for table in [
        "plugins",
        "plugin_permissions",
        "plugin_config",
        "plugin_storage",
    ] {
        let left: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE plugin_id = 'com.example.e2e'"),
                rusqlite::params![],
                |r| r.get(0),
            )
            .expect("查残留");
        assert_eq!(left, 0, "{table} 卸载后不得有残留行");
    }
    // 宿主设置与内置别名不属于插件，不受卸载影响
    assert_eq!(
        settings::get(&pool, "window.layout")
            .expect("读设置")
            .as_deref(),
        Some("{\"w\":1024,\"h\":768}")
    );
    assert_eq!(vdirs::list(&pool).expect("列别名").len(), 3);

    drop(conn);
    drop(pool);
    drop(dir); // TempDir 清理（Windows 下先释放连接）
}

/// 5.2：写入数据后重启，数据完整且迁移不重复执行。
#[test]
fn data_survives_restart_and_migrations_do_not_rerun() {
    let (_dir, path) = common::temp_db();

    // 第一次启动：写入
    {
        let pool = common::opened(&path);
        common::register_plugin(&pool, "com.example.persist");
        config::set_value(&pool, "com.example.persist", "level", "7").expect("写配置");
        storage::set_plugin_key(&pool, "com.example.persist", "cache/blob", &vec![9; 5000])
            .expect("写私有数据");
        settings::set(&pool, "ui.scale", "1.25").expect("写设置");
        storage::set_reserved_key(&pool, "com.example.persist", "__picked/p", b"delivered")
            .expect("宿主交付");
    }

    // 第二次启动：显式分步调用以观察迁移结果
    let pool = tessera_store::pool::open_pool(&path, tessera_store::pool::DEFAULT_MAX_POOL_SIZE)
        .expect("重开池");
    let outcome = migrate::run(&path, &pool).expect("再次迁移");
    assert!(outcome.applied.is_empty(), "重启不得重复执行已应用的迁移");
    assert!(outcome.backup.is_none());

    let records: i64 = pool
        .get()
        .expect("取连接")
        .query_row(
            "SELECT count(*) FROM schema_migrations",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("查迁移记录");
    assert_eq!(records, 1, "schema_migrations 未新增记录");

    // 数据完整一致
    let rec = plugins::get(&pool, "com.example.persist")
        .expect("查插件")
        .expect("在");
    assert!(rec.enabled);
    assert_eq!(
        config::get_value(&pool, "com.example.persist", "level").expect("读配置"),
        Some("7".to_string())
    );
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.persist", "cache/blob").expect("读私有数据"),
        Some(vec![9; 5000])
    );
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.persist", "__picked/p").expect("读交付条目"),
        Some(b"delivered".to_vec())
    );
    assert_eq!(
        settings::get(&pool, "ui.scale").expect("读设置").as_deref(),
        Some("1.25")
    );
    let usage = storage::usage(&pool, "com.example.persist").expect("用量");
    assert_eq!((usage.bytes, usage.keys), (5000, 1), "计数器跨重启持久");
}
