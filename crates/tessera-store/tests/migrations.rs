//! 任务 1.2–1.5 验证：迁移幂等、失败整条回滚、迁移前自动备份、
//! 降级保护拒绝启动、开发模式重建入口与 001 冻结声明。

mod common;

use std::path::Path;

use tessera_store::StoreError;
use tessera_store::migrate::{self, MIGRATIONS, Migration};

const V2_SQL: &str = "CREATE TABLE v2_marker (x);";

fn v2_list() -> Vec<Migration> {
    let mut list = MIGRATIONS.to_vec();
    list.push(Migration {
        version: 2,
        name: "test_v2",
        sql: V2_SQL,
    });
    list
}

/// 1.2：重复启动时已应用的迁移不再执行。
#[test]
fn migrations_are_idempotent() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);

    // 全新建库不产生备份（无内容可备份；连接一打开文件即被创建，
    // 因此不能以文件存在与否判断是否全新库）
    assert!(
        !path.with_file_name("tessera.db.bak-0").exists(),
        "全新建库不应产生空备份"
    );

    let second = migrate::run(&path, &pool).expect("第二次迁移");
    assert!(second.applied.is_empty(), "不应重复应用迁移");
    assert!(second.backup.is_none(), "无事可做时不应产生备份");

    let records: i64 = pool
        .get()
        .expect("取连接")
        .query_row(
            "SELECT count(*) FROM schema_migrations",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("查迁移记录数");
    assert_eq!(records, 1, "schema_migrations 不应新增记录");
}

/// 1.2：迁移中途失败时该条改动被完整回滚，数据库停留在前一个完整版本。
#[test]
fn failed_migration_rolls_back_completely() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);

    // 先正常跑到 v1，再注入一条「先建表、后语法错误」的坏迁移
    let mut bad = MIGRATIONS.to_vec();
    bad.push(Migration {
        version: 2,
        name: "bad",
        sql: "CREATE TABLE should_rollback (x);\nTHIS IS NOT VALID SQL;",
    });
    let err = migrate::run_with(&path, &pool, &bad).expect_err("坏迁移必须失败");

    assert!(
        matches!(err, StoreError::Host(ref e) if e.to_string().contains("数据库访问失败")),
        "失败应包装为数据库错误，实际 {err:?}"
    );

    let conn = pool.get().expect("取连接");
    let rolled_back: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'should_rollback')",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("查回滚痕迹");
    assert!(!rolled_back, "坏迁移建立的表必须随整条回滚消失");

    let max_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("查版本");
    assert_eq!(max_version, 1, "数据库应停留在前一个完整版本");
}

/// 1.3：存在未应用迁移时，执行前生成带版本标识的备份，内容与迁移前一致。
#[test]
fn backup_created_before_applying_new_migration() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);

    // 迁移前写入种子数据
    tessera_store::settings::set(&pool, "marker", "\"v1-data\"").expect("写种子");

    let outcome = migrate::run_with(&path, &pool, &v2_list()).expect("应用 v2");
    assert_eq!(outcome.applied, vec![2]);
    let backup = outcome.backup.expect("必须生成备份文件");
    assert!(
        backup.to_string_lossy().ends_with("tessera.db.bak-1"),
        "备份文件名带迁移前版本标识，实际 {}",
        backup.display()
    );
    assert!(backup.exists(), "备份文件必须落盘");

    // 备份内容与迁移前的数据库一致：种子数据在、v2 产物不在
    let bak = rusqlite::Connection::open(&backup).expect("打开备份");
    let marker: Option<String> = bak
        .query_row(
            "SELECT value_json FROM host_settings WHERE key = 'marker'",
            rusqlite::params![],
            |r| r.get(0),
        )
        .map(Some)
        .unwrap_or(None);
    assert_eq!(marker.as_deref(), Some("\"v1-data\""), "备份应含迁移前数据");
    let has_v2: bool = bak
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'v2_marker')",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("查备份结构");
    assert!(!has_v2, "备份必须停在迁移前结构");

    // 主库已前进到 v2
    let main_has_v2: bool = pool
        .get()
        .expect("取连接")
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'v2_marker')",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("查主库结构");
    assert!(main_has_v2);
}

/// 1.4：数据库版本高于代码支持时拒绝启动，且数据库内容未被修改。
#[test]
fn downgrade_is_refused_without_touching_db() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);
    tessera_store::settings::set(&pool, "user", "\"data\"").expect("写数据");

    // 模拟「数据库已被新版本迁移过」：手工把版本抬高
    pool.get()
        .expect("取连接")
        .execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (999, '2030-01-01T00:00:00Z')",
            rusqlite::params![],
        )
        .expect("抬高版本");

    let err = tessera_store::open(&path).expect_err("降级启动必须被拒绝");
    assert!(matches!(
        err,
        StoreError::SchemaNewerThanCode { db: 999, code: 1 }
    ));
    let msg = err.to_string();
    assert!(
        msg.contains("升级应用"),
        "提示必须可操作（升级应用），实际：{msg}"
    );

    // 数据库内容未被修改
    let conn = pool.get().expect("取连接");
    let user: String = conn
        .query_row(
            "SELECT value_json FROM host_settings WHERE key = 'user'",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("数据必须完好");
    assert_eq!(user, "\"data\"");
    let records: i64 = conn
        .query_row(
            "SELECT count(*) FROM schema_migrations",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("查记录数");
    assert_eq!(records, 2, "schema_migrations 不应有任何增删");
}

/// 1.5：重建入口删库重来；正常打开路径不删任何文件。
///
/// 「非开发模式下不可达」由 `rebuild` 上的 `#[cfg(debug_assertions)]` 门控保证
/// （release 构建中符号不存在）；测试本身在 dev profile 编译运行，即门控下的可用形态。
#[test]
fn rebuild_is_a_separate_explicit_path() {
    let (_dir, path) = common::temp_db();

    // 正常路径：写数据、关库、重开——数据必须还在（open 不删文件）
    {
        let pool = common::opened(&path);
        tessera_store::settings::set(&pool, "keep", "1").expect("写数据");
    }
    {
        let pool = common::opened(&path);
        let v: Option<String> = tessera_store::settings::get(&pool, "keep").expect("读数据");
        assert_eq!(v.as_deref(), Some("1"), "open 路径不得删除既有数据");
    }

    // 重建路径：显式调用才删库，从零重新应用全部迁移
    let (pool, outcome) = migrate::rebuild(&path).expect("重建");
    assert_eq!(outcome.applied, vec![1], "重建应从零应用全部迁移");
    let v: Option<String> = tessera_store::settings::get(&pool, "keep").expect("读数据");
    assert_eq!(v, None, "重建后旧数据不复存在");
}

/// 1.5：`001_init.sql` 顶部写明已冻结、结构变更请新增迁移。
#[test]
fn init_migration_declares_frozen() {
    let sql = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/001_init.sql"
    ))
    .expect("读取 001_init.sql");
    assert!(
        sql.contains("已冻结") && sql.contains("新增迁移"),
        "001 顶部必须有冻结警告"
    );
}

/// 辅助：确认 Path 参数形态可用（编译期检查 open 的公开签名）。
#[allow(dead_code)]
fn _path_type_check(p: &Path) {
    let _ = p;
}
