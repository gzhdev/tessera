//! 任务 2.1–2.5 验证：§11.2 全部表与字段、外键级联、内置别名种子、计数列初始 0。

mod common;

use tessera_store::pool::DbPool;

/// 某表按序的列名列表（PRAGMA table_info 的 cid 顺序）。
fn columns(pool: &DbPool, table: &str) -> Vec<String> {
    let conn = pool.get().expect("取连接");
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("查表结构");

    stmt.query_map(rusqlite::params![], |r| r.get::<_, String>(1))
        .expect("枚举列")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("收集列名")
}

/// 2.1 / 2.2 / 2.3：全部七张表存在且字段与设计书 §11.2 一致。
#[test]
fn all_tables_match_design() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);

    assert_eq!(
        columns(&pool, "schema_migrations"),
        ["version", "applied_at"]
    );
    assert_eq!(
        columns(&pool, "plugins"),
        [
            "plugin_id",
            "version",
            "install_path",
            "manifest_json",
            "enabled",
            "installed_at",
            "last_activated_at",
            "storage_bytes",
            "storage_keys",
        ]
    );
    assert_eq!(
        columns(&pool, "plugin_permissions"),
        [
            "plugin_id",
            "permission_type",
            "scope_json",
            "granted",
            "decided_at"
        ]
    );
    assert_eq!(
        columns(&pool, "virtual_dirs"),
        ["alias", "real_path", "writable", "builtin"]
    );
    assert_eq!(
        columns(&pool, "plugin_config"),
        ["plugin_id", "key", "value_json", "updated_at"]
    );
    assert_eq!(
        columns(&pool, "plugin_storage"),
        ["plugin_id", "key", "value", "updated_at"]
    );
    assert_eq!(
        columns(&pool, "host_settings"),
        ["key", "value_json", "updated_at"]
    );

    // §11.2 的插件存储索引
    let has_index: bool = pool
        .get()
        .expect("取连接")
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_plugin_storage_plugin')",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("查索引");
    assert!(has_index, "idx_plugin_storage_plugin 必须存在");
}

/// 2.2：删除 plugins 记录后，三张从属表无孤儿行（外键 ON DELETE CASCADE）。
#[test]
fn deleting_plugin_cascades_to_dependent_tables() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.a");

    tessera_store::permissions::grant(
        &pool,
        "com.example.a",
        "fs.read",
        "{\"virtualDir\":\"workspace\"}",
    )
    .expect("授权");
    tessera_store::config::set_value(&pool, "com.example.a", "theme", "\"dark\"").expect("写配置");
    tessera_store::storage::set_plugin_key(&pool, "com.example.a", "k", b"v").expect("写存储");
    tessera_store::storage::set_reserved_key(&pool, "com.example.a", "__picked/t", b"file")
        .expect("写交付条目");

    tessera_store::plugins::uninstall(&pool, "com.example.a").expect("卸载");

    for table in ["plugin_permissions", "plugin_config", "plugin_storage"] {
        let orphans: i64 = pool
            .get()
            .expect("取连接")
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE plugin_id = 'com.example.a'"),
                rusqlite::params![],
                |r| r.get(0),
            )
            .expect("查孤儿行");
        assert_eq!(orphans, 0, "{table} 不得残留该插件的行");
    }
}

/// 2.4：三个内置别名随首次建库插入且 builtin 为真；未映射（空路径）是合法状态。
#[test]
fn builtin_aliases_are_seeded() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);

    let list = tessera_store::vdirs::list(&pool).expect("列别名");
    let by_alias: Vec<(String, bool, bool)> = list
        .into_iter()
        .map(|d| (d.alias, d.writable, d.builtin))
        .collect();
    assert_eq!(
        by_alias,
        vec![
            ("plugin-data".to_string(), true, true),
            ("temp".to_string(), false, true),
            ("workspace".to_string(), false, true),
        ],
        "三个内置别名存在、builtin 为真；plugin-data 按 §7.2 自动授予读写"
    );
}

/// 2.5：新建插件记录时计数列初始为 0。
#[test]
fn storage_counters_start_at_zero() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.fresh");

    let rec = tessera_store::plugins::get(&pool, "com.example.fresh")
        .expect("查询")
        .expect("已登记");
    assert_eq!(rec.storage_bytes, 0);
    assert_eq!(rec.storage_keys, 0);
}
