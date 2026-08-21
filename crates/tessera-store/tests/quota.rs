//! 任务 4.1–4.8 验证：三重配额边界、配额按插件独立、`__` 前缀拒写、
//! 宿主保留键不占配额、交付条目上限与淘汰、停用清除、用量查询与手动清理。

mod common;

use tessera_store::StoreError;
use tessera_store::storage::{self, QUOTA_MAX_KEYS, QUOTA_TOTAL_BYTES, QUOTA_VALUE_BYTES};

/// 4.1：写入后计数列与 SUM(length(value)) 一致；覆盖写入按差值更新。
#[test]
fn counters_track_actual_bytes_and_keys() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.count");

    storage::set_plugin_key(&pool, "com.example.count", "a", &[1; 100]).expect("写 a");
    storage::set_plugin_key(&pool, "com.example.count", "b", &[2; 50]).expect("写 b");

    let usage = storage::usage(&pool, "com.example.count").expect("用量");
    assert_eq!((usage.bytes, usage.keys), (150, 2));
    let actual_sum = common::scalar_i64(
        &pool,
        "SELECT COALESCE(SUM(length(value)), 0) FROM plugin_storage WHERE plugin_id = ?1",
        "com.example.count",
    );
    assert_eq!(usage.bytes, actual_sum, "计数列必须与实际 SUM 一致");

    // 覆盖 a：100 → 30 字节，差值扣减
    storage::set_plugin_key(&pool, "com.example.count", "a", &[1; 30]).expect("覆盖 a");
    let usage = storage::usage(&pool, "com.example.count").expect("用量");
    assert_eq!(
        (usage.bytes, usage.keys),
        (80, 2),
        "覆盖写入按差值计入，键数不变"
    );

    // 删除 b：字节与键数同步扣减
    assert!(storage::delete_plugin_key(&pool, "com.example.count", "b").expect("删 b"));
    let usage = storage::usage(&pool, "com.example.count").expect("用量");
    assert_eq!((usage.bytes, usage.keys), (30, 1));
}

/// 4.2：三条配额各有一个边界用例；超限时已有数据完好。
#[test]
fn quota_boundaries_reject_without_damage() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.quota");
    storage::set_plugin_key(&pool, "com.example.quota", "keep", b"precious").expect("写种子");

    // 边界 1：单值上限——恰为 1 MB 允许，超出 1 字节拒绝
    storage::set_plugin_key(
        &pool,
        "com.example.quota",
        "max-value",
        &vec![0; QUOTA_VALUE_BYTES],
    )
    .expect("恰好 1 MB 的单值应被允许");
    let err = storage::set_plugin_key(
        &pool,
        "com.example.quota",
        "too-big",
        &vec![0; QUOTA_VALUE_BYTES + 1],
    )
    .expect_err("超 1 MB 单值必须被拒");
    assert!(err.is_quota(), "应返回配额超限错误，实际 {err:?}");

    // 边界 2：总量 16 MB——伪造计数器逼近上限（计数器是配额判断权威，D5）
    pool.get()
        .expect("取连接")
        .execute(
            "UPDATE plugins SET storage_bytes = ?2 WHERE plugin_id = ?1",
            rusqlite::params!["com.example.quota", QUOTA_TOTAL_BYTES - 10],
        )
        .expect("伪造已用量");
    storage::set_plugin_key(&pool, "com.example.quota", "fits-exactly", &[0; 10])
        .expect("恰好填满 16 MB 应被允许");
    let err = storage::set_plugin_key(&pool, "com.example.quota", "over-total", &[0; 11])
        .expect_err("总量超限必须被拒");
    assert!(err.is_quota());

    // 边界 3：键数 10000——恰满允许，第 10001 个键拒绝
    pool.get()
        .expect("取连接")
        .execute(
            "UPDATE plugins SET storage_keys = ?2, storage_bytes = 0 WHERE plugin_id = ?1",
            rusqlite::params!["com.example.quota", QUOTA_MAX_KEYS - 1],
        )
        .expect("伪造键数");
    storage::set_plugin_key(&pool, "com.example.quota", "key-10000", b"x")
        .expect("第 10000 个键应被允许");
    let err = storage::set_plugin_key(&pool, "com.example.quota", "key-10001", b"x")
        .expect_err("第 10001 个键必须被拒");
    assert!(err.is_quota());

    // 超限后已有数据完好
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.quota", "keep").expect("读种子"),
        Some(b"precious".to_vec())
    );
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.quota", "fits-exactly").expect("读边界值"),
        Some(vec![0; 10])
    );
}

/// 4.3：配额按插件独立——A 用满后 B 的写入不受影响。
#[test]
fn quotas_are_per_plugin() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.full");
    common::register_plugin(&pool, "com.example.other");

    pool.get()
        .expect("取连接")
        .execute(
            "UPDATE plugins SET storage_bytes = ?2 WHERE plugin_id = ?1",
            rusqlite::params!["com.example.full", QUOTA_TOTAL_BYTES],
        )
        .expect("A 用满配额");

    let err = storage::set_plugin_key(&pool, "com.example.full", "new", b"x")
        .expect_err("A 的写入必须被拒");
    assert!(err.is_quota());
    storage::set_plugin_key(&pool, "com.example.other", "new", b"x").expect("B 的写入不受 A 影响");
}

/// 4.4：`__` 前缀拒写（参数非法）；`_internal` 与 `cache__data` 正常。
#[test]
fn double_underscore_prefix_is_rejected_for_plugins() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.prefix");

    let err = storage::set_plugin_key(&pool, "com.example.prefix", "__picked/abc", b"v")
        .expect_err("插件写保留键必须被拒");
    assert!(
        matches!(err, StoreError::InvalidArgument(_)),
        "应返回参数非法，实际 {err:?}"
    );
    assert!(matches!(
        storage::delete_plugin_key(&pool, "com.example.prefix", "__anything"),
        Err(StoreError::InvalidArgument(_))
    ));

    storage::set_plugin_key(&pool, "com.example.prefix", "_internal", b"v")
        .expect("_internal 合法");
    storage::set_plugin_key(&pool, "com.example.prefix", "cache__data", b"v")
        .expect("cache__data 合法");
    // 不以 __ 开头就不受影响
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.prefix", "_internal").expect("读"),
        Some(b"v".to_vec())
    );
}

/// 4.5：宿主写保留键（30 MB）不占插件配额；插件仍能正常写自己的数据。
/// 同时验证「插件可读保留键」。
#[test]
fn host_reserved_writes_do_not_consume_quota() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.host");

    storage::set_plugin_key(&pool, "com.example.host", "own", b"mine").expect("先写自己的数据");
    let before = storage::usage(&pool, "com.example.host").expect("用量");

    let big = vec![7; 30 * 1024 * 1024];
    storage::set_reserved_key(&pool, "com.example.host", "__picked/token1", &big)
        .expect("宿主写入 30 MB 交付内容");

    let after = storage::usage(&pool, "com.example.host").expect("用量");
    assert_eq!(after, before, "保留命名空间不得计入插件配额");

    // 插件可读保留键（spec storage-quota「插件可读保留键」）
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.host", "__picked/token1").expect("读"),
        Some(big)
    );
    // 插件仍能正常写自己的数据
    storage::set_plugin_key(&pool, "com.example.host", "own2", b"more").expect("配额不受挤占");

    // 宿主接口不接受非保留键（D6：误用编译期不可达之外再挡一层运行期）
    assert!(matches!(
        storage::set_reserved_key(&pool, "com.example.host", "plain", b"v"),
        Err(StoreError::InvalidArgument(_))
    ));
}

/// 4.6：交付条目上限——存活不超过 4 条，写出第 5 条淘汰最旧；覆盖不触发淘汰；
/// 单条超过单次文件读取上限被拒。
#[test]
fn picked_entries_evict_oldest_beyond_four() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.pick");

    let keys = |pool: &tessera_store::pool::DbPool| -> Vec<String> {
        let conn = pool.get().expect("取连接");
        let mut stmt = conn
            .prepare("SELECT key FROM plugin_storage WHERE plugin_id = ?1 ORDER BY rowid")
            .expect("查键");
        stmt.query_map(rusqlite::params!["com.example.pick"], |r| {
            r.get::<_, String>(0)
        })
        .expect("枚举")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("收集")
    };

    for token in ["t1", "t2", "t3", "t4"] {
        storage::set_reserved_key(
            &pool,
            "com.example.pick",
            &format!("__picked/{token}"),
            b"content",
        )
        .expect("写交付条目");
    }
    assert_eq!(keys(&pool).len(), 4, "存活 4 条");

    // 覆盖非最新条目：不触发淘汰，仍 4 条
    storage::set_reserved_key(&pool, "com.example.pick", "__picked/t2", b"updated")
        .expect("覆盖已有条目");
    assert_eq!(keys(&pool).len(), 4, "覆盖不得触发淘汰");

    // 第 5 条（新键）：最旧的 t1 被淘汰，写入成功
    storage::set_reserved_key(&pool, "com.example.pick", "__picked/t5", b"newest")
        .expect("第 5 条写入成功");
    let remaining = keys(&pool);
    assert_eq!(remaining.len(), 4, "淘汰后仍为 4 条");
    assert!(
        !remaining.contains(&"__picked/t1".to_string()),
        "最旧一条必须消失"
    );
    for k in ["__picked/t2", "__picked/t3", "__picked/t4", "__picked/t5"] {
        assert!(remaining.contains(&k.to_string()), "{k} 应存活");
    }

    // 单条超过 64 MB 被拒
    let err = storage::set_reserved_key(
        &pool,
        "com.example.pick",
        "__picked/huge",
        &vec![0; storage::MAX_FILE_READ_BYTES + 1],
    )
    .expect_err("单条超限必须被拒");
    assert!(matches!(err, StoreError::InvalidArgument(_)));
    assert!(
        storage::get_plugin_key(&pool, "com.example.pick", "__picked/huge")
            .expect("读")
            .is_none()
    );
}

/// 4.7：停用清除全部 `__picked/*` 而保留普通键值；无时间过期。
#[test]
fn deactivation_clears_picked_but_keeps_private_data() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.deact");

    storage::set_plugin_key(&pool, "com.example.deact", "state", b"keep-me").expect("普通键");
    storage::set_reserved_key(&pool, "com.example.deact", "__picked/a", b"f1").expect("交付 a");
    storage::set_reserved_key(&pool, "com.example.deact", "__picked/b", b"f2").expect("交付 b");

    // 停用：由生命周期层调用清除入口（存储层提供该能力）
    tessera_store::plugins::set_enabled(&pool, "com.example.deact", false).expect("停用");
    let cleared = storage::clear_picked_entries(&pool, "com.example.deact").expect("清除交付条目");
    assert_eq!(cleared, 2);

    assert!(
        storage::get_plugin_key(&pool, "com.example.deact", "__picked/a")
            .expect("读")
            .is_none()
    );
    assert!(
        storage::get_plugin_key(&pool, "com.example.deact", "__picked/b")
            .expect("读")
            .is_none()
    );
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.deact", "state").expect("读"),
        Some(b"keep-me".to_vec()),
        "普通键值数据在停用后保留"
    );

    // 「不设时间过期」：存储层不存在任何按时间清除交付条目的路径——
    // 条目一旦写入，读取的唯一清除途径是停用或淘汰（上面已各自验证）。
    // 此处再断言一次「很久之后」语义下读取仍成功：重新交付一条，随即读取。
    storage::set_reserved_key(&pool, "com.example.deact", "__picked/c", b"f3").expect("再交付");
    assert_eq!(
        storage::get_plugin_key(&pool, "com.example.deact", "__picked/c").expect("读"),
        Some(b"f3".to_vec())
    );
}

/// 4.8：用量查询不含保留命名空间；手动清理后用量归零、可重新写入，
/// 且清理顺带重算计数器。
#[test]
fn usage_excludes_reserved_and_manual_clear_resets() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    common::register_plugin(&pool, "com.example.clean");

    storage::set_plugin_key(&pool, "com.example.clean", "data", &[1; 100]).expect("普通键");
    storage::set_reserved_key(&pool, "com.example.clean", "__picked/f", &vec![2; 4096])
        .expect("交付条目");
    storage::set_reserved_key(&pool, "com.example.clean", "__meta", b"x").expect("其他保留键");

    let usage = storage::usage(&pool, "com.example.clean").expect("用量");
    assert_eq!(
        (usage.bytes, usage.keys),
        (100, 1),
        "用量不得包含保留命名空间"
    );

    // 手动清理：普通键清空、计数归零；保留键归宿主管辖，不随用户清理消失
    storage::clear_plugin_storage(&pool, "com.example.clean").expect("清理");
    let usage = storage::usage(&pool, "com.example.clean").expect("用量");
    assert_eq!((usage.bytes, usage.keys), (0, 0), "清理后用量必须归零");
    assert!(
        storage::get_plugin_key(&pool, "com.example.clean", "__picked/f")
            .expect("读")
            .is_some()
    );

    // 可重新写入
    storage::set_plugin_key(&pool, "com.example.clean", "again", b"fresh").expect("重新写入");

    // recalc：外部直改库造成的漂移被纠正（保留键不计入）
    pool.get()
        .expect("取连接")
        .execute(
            "UPDATE plugins SET storage_bytes = 999999, storage_keys = 999 WHERE plugin_id = ?1",
            rusqlite::params!["com.example.clean"],
        )
        .expect("制造漂移");
    let usage = storage::recalc_usage(&pool, "com.example.clean").expect("重算");
    assert_eq!((usage.bytes, usage.keys), (5, 1), "重算只统计非保留键");
}
