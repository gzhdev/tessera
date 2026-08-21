//! 任务 3.1–3.4 验证：plugins 增删改查、权限与别名访问层、配置与设置的
//! 读写覆盖、配置值与私有数据的分离。

mod common;

use tessera_store::StoreError;

/// 3.1：plugins 表访问层增删改查。
#[test]
fn plugins_register_query_toggle_touch_uninstall() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.p1");

    let rec = tessera_store::plugins::get(&pool, "com.example.p1")
        .expect("查询")
        .expect("存在");
    assert_eq!(rec.version, "0.1.0");
    assert_eq!(rec.install_path, "/plugins/com.example.p1");
    assert_eq!(rec.manifest_json, "{}");
    assert!(rec.enabled, "登记后默认启用");
    assert!(rec.last_activated_at.is_none());
    assert!(!rec.installed_at.is_empty());

    // 重复登记被拒
    let dup = tessera_store::plugins::register(
        &pool,
        &tessera_store::plugins::NewPlugin {
            plugin_id: "com.example.p1".into(),
            version: "0.2.0".into(),
            install_path: String::new(),
            manifest_json: String::new(),
        },
    );
    assert!(matches!(dup, Err(StoreError::InvalidArgument(_))));

    tessera_store::plugins::set_enabled(&pool, "com.example.p1", false).expect("停用");
    assert!(
        !tessera_store::plugins::get(&pool, "com.example.p1")
            .expect("查")
            .expect("在")
            .enabled
    );

    tessera_store::plugins::touch_last_activated(&pool, "com.example.p1").expect("激活时间");
    let rec = tessera_store::plugins::get(&pool, "com.example.p1")
        .expect("查")
        .expect("在");
    assert!(rec.last_activated_at.is_some(), "激活时间应被记录");

    assert_eq!(tessera_store::plugins::list(&pool).expect("列表").len(), 1);

    tessera_store::plugins::uninstall(&pool, "com.example.p1").expect("卸载");
    assert!(
        tessera_store::plugins::get(&pool, "com.example.p1")
            .expect("查")
            .is_none()
    );

    // 对不存在插件的操作
    assert!(matches!(
        tessera_store::plugins::set_enabled(&pool, "com.example.gone", true),
        Err(StoreError::NotFound(_))
    ));
    assert!(matches!(
        tessera_store::plugins::uninstall(&pool, "com.example.gone"),
        Err(StoreError::NotFound(_))
    ));
}

/// 3.1（续）：卸载时级联清除从属数据（经访问层观察）。
#[test]
fn uninstall_clears_dependents_via_access_layer() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.c1");

    tessera_store::permissions::grant(
        &pool,
        "com.example.c1",
        "net.http",
        "{\"allowedDomains\":[\"example.com\"]}",
    )
    .expect("授权");
    tessera_store::config::set_value(&pool, "com.example.c1", "k", "1").expect("写配置");
    tessera_store::storage::set_plugin_key(&pool, "com.example.c1", "s", b"v").expect("写存储");

    tessera_store::plugins::uninstall(&pool, "com.example.c1").expect("卸载");

    assert!(
        tessera_store::permissions::list_for_plugin(&pool, "com.example.c1")
            .expect("列权限")
            .is_empty()
    );
    assert!(
        tessera_store::config::list_for_plugin(&pool, "com.example.c1")
            .expect("列配置")
            .is_empty()
    );
    assert!(
        tessera_store::storage::get_plugin_key(&pool, "com.example.c1", "s")
            .expect("读存储")
            .is_none()
    );
}

/// 3.2：权限授予 / 幂等 / 撤销；内置别名删除保护与自建别名自由增删。
#[test]
fn permissions_and_vdirs_protection() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.perm");

    let scope = "{\"virtualDir\":\"workspace\"}";
    tessera_store::permissions::grant(&pool, "com.example.perm", "fs.read", scope).expect("授权");
    tessera_store::permissions::grant(&pool, "com.example.perm", "fs.read", scope)
        .expect("重复授予幂等");
    let grants =
        tessera_store::permissions::list_for_plugin(&pool, "com.example.perm").expect("列权限");
    assert_eq!(grants.len(), 1, "重复授予不得产生第二条记录");
    assert!(grants[0].granted);

    tessera_store::permissions::revoke(&pool, "com.example.perm", "fs.read", scope).expect("撤销");
    let grants =
        tessera_store::permissions::list_for_plugin(&pool, "com.example.perm").expect("列权限");
    assert_eq!(grants.len(), 1, "撤销保留审计记录");
    assert!(!grants[0].granted);

    assert!(matches!(
        tessera_store::permissions::revoke(
            &pool,
            "com.example.perm",
            "fs.read",
            "{\"virtualDir\":\"temp\"}"
        ),
        Err(StoreError::NotFound(_))
    ));

    // 内置别名不可删；自建别名可自由增删
    assert!(matches!(
        tessera_store::vdirs::delete(&pool, "workspace"),
        Err(StoreError::Rejected(_))
    ));
    assert!(matches!(
        tessera_store::vdirs::delete(&pool, "plugin-data"),
        Err(StoreError::Rejected(_))
    ));

    // 内置别名也不可经自建入口改写（upsert 的 builtin 保护与 delete 对称），
    // 且被拒后原行原样保留
    assert!(matches!(
        tessera_store::vdirs::upsert(&pool, "temp", "D:/evil", true),
        Err(StoreError::Rejected(_))
    ));
    let temp = tessera_store::vdirs::get(&pool, "temp")
        .expect("查")
        .expect("在");
    assert_eq!(temp.real_path, "", "内置别名的映射不得被改写");
    assert!(!temp.writable, "内置别名的 writable 不得被改写");

    tessera_store::vdirs::upsert(&pool, "my-project", "D:/work/project", true).expect("自建别名");
    let dir = tessera_store::vdirs::get(&pool, "my-project")
        .expect("查别名")
        .expect("存在");
    assert!(dir.writable && !dir.builtin);
    tessera_store::vdirs::upsert(&pool, "my-project", "E:/other", false).expect("覆盖更新");
    assert_eq!(
        tessera_store::vdirs::get(&pool, "my-project")
            .expect("查")
            .expect("在")
            .real_path,
        "E:/other"
    );
    tessera_store::vdirs::delete(&pool, "my-project").expect("自建别名可删除");
    assert!(
        tessera_store::vdirs::get(&pool, "my-project")
            .expect("查")
            .is_none()
    );

    // workspace 未映射 → 空串（合法状态）；填充映射走 set_real_path
    assert_eq!(
        tessera_store::vdirs::get(&pool, "workspace")
            .expect("查")
            .expect("在")
            .real_path,
        ""
    );
    tessera_store::vdirs::set_real_path(&pool, "workspace", "D:/docs").expect("填充映射");
    assert_eq!(
        tessera_store::vdirs::get(&pool, "workspace")
            .expect("查")
            .expect("在")
            .real_path,
        "D:/docs"
    );
    assert!(matches!(
        tessera_store::vdirs::set_real_path(&pool, "no-such", "x"),
        Err(StoreError::NotFound(_))
    ));
}

/// 3.3：plugin_config 与 host_settings 的读写与覆盖写。
#[test]
fn config_and_settings_read_write_overwrite() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.cfg");

    // 配置：写 → 读 → 覆盖 → 列 → 删
    tessera_store::config::set_value(&pool, "com.example.cfg", "theme", "\"dark\"")
        .expect("写配置");
    assert_eq!(
        tessera_store::config::get_value(&pool, "com.example.cfg", "theme").expect("读"),
        Some("\"dark\"".to_string())
    );
    tessera_store::config::set_value(&pool, "com.example.cfg", "theme", "\"light\"")
        .expect("覆盖写");
    assert_eq!(
        tessera_store::config::get_value(&pool, "com.example.cfg", "theme").expect("读"),
        Some("\"light\"".to_string())
    );
    tessera_store::config::set_value(&pool, "com.example.cfg", "level", "3").expect("写第二项");
    assert_eq!(
        tessera_store::config::list_for_plugin(&pool, "com.example.cfg").expect("列配置"),
        vec![
            ("level".to_string(), "3".to_string()),
            ("theme".to_string(), "\"light\"".to_string())
        ]
    );
    assert!(tessera_store::config::delete_value(&pool, "com.example.cfg", "theme").expect("删"));
    assert!(
        tessera_store::config::get_value(&pool, "com.example.cfg", "theme")
            .expect("读")
            .is_none()
    );

    // 未登记插件写配置 → 外键拒绝，参数非法
    assert!(matches!(
        tessera_store::config::set_value(&pool, "com.example.ghost", "k", "1"),
        Err(StoreError::InvalidArgument(_))
    ));

    // 设置：写 → 覆盖 → 删
    tessera_store::settings::set(&pool, "window.layout", "{\"w\":800}").expect("写设置");
    assert_eq!(
        tessera_store::settings::get(&pool, "window.layout").expect("读"),
        Some("{\"w\":800}".to_string())
    );
    tessera_store::settings::set(&pool, "window.layout", "{\"w\":1024}").expect("覆盖");
    assert_eq!(
        tessera_store::settings::get(&pool, "window.layout").expect("读"),
        Some("{\"w\":1024}".to_string())
    );
    assert!(tessera_store::settings::delete(&pool, "window.layout").expect("删"));
    assert!(
        tessera_store::settings::get(&pool, "window.layout")
            .expect("读")
            .is_none()
    );
    assert_eq!(
        tessera_store::settings::list(&pool).expect("列设置").len(),
        0
    );
}

/// 3.4：配置值与私有数据分表存放，互不干扰。
#[test]
fn config_values_survive_heavy_storage_writes() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.mix");

    tessera_store::config::set_value(&pool, "com.example.mix", "apiKey", "\"secret\"")
        .expect("写配置");

    // 写大量私有数据（32 × 64 KB = 2 MB）
    let blob = vec![0xAB_u8; 64 * 1024];
    for i in 0..32 {
        tessera_store::storage::set_plugin_key(
            &pool,
            "com.example.mix",
            &format!("cache/{i}"),
            &blob,
        )
        .expect("写私有数据");
    }

    // 配置值读取结果不变；两表行数各自独立
    assert_eq!(
        tessera_store::config::get_value(&pool, "com.example.mix", "apiKey").expect("读配置"),
        Some("\"secret\"".to_string())
    );
    assert_eq!(
        tessera_store::config::list_for_plugin(&pool, "com.example.mix")
            .expect("列配置")
            .len(),
        1
    );
    assert_eq!(
        common::scalar_i64(
            &pool,
            "SELECT count(*) FROM plugin_storage WHERE plugin_id = ?1",
            "com.example.mix"
        ),
        32
    );
}
