//! 任务 1.1 验证：WAL、外键、`synchronous = NORMAL` 生效；
//! 一个写事务进行时其他线程的读正常完成，不被阻塞。

mod common;

use std::thread;

#[test]
fn connection_pragmas_are_set() {
    let (_dir, path) = common::temp_db();
    let pool = common::opened(&path);

    let conn = pool.get().expect("取连接");
    let journal: String = conn
        .query_row("PRAGMA journal_mode", rusqlite::params![], |r| r.get(0))
        .expect("journal_mode");
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys", rusqlite::params![], |r| r.get(0))
        .expect("foreign_keys");
    let sync: i64 = conn
        .query_row("PRAGMA synchronous", rusqlite::params![], |r| r.get(0))
        .expect("synchronous");

    assert_eq!(journal.to_lowercase(), "wal");
    assert_eq!(fk, 1, "外键约束必须开启，级联删除才生效");
    assert_eq!(sync, 1, "synchronous 必须为 NORMAL（1）");
}

#[test]
fn reader_not_blocked_while_write_transaction_open() {
    let (_dir, path) = common::temp_db();
    let pool = tessera_store::pool::open_pool(&path, 4).expect("建池");
    tessera_store::migrate::run(&path, &pool).expect("迁移");

    // 种子数据，让「读到旧值」有明确内容
    tessera_store::settings::set(&pool, "seed", "1").expect("写种子");

    // 连接 1：开写事务、写入、保持未提交
    let mut writer = pool.get().expect("取写连接");
    let tx = writer
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .expect("开写事务");
    tx.execute(
        "INSERT INTO host_settings (key, value_json, updated_at) VALUES ('pending', '2', '2026-01-01T00:00:00Z')",
        rusqlite::params![],
    )
    .expect("事务内写入");

    // 连接 2（另一线程）：读必须在写事务未提交期间完成并看到旧快照
    let reader_pool = pool.clone();
    let handle = thread::spawn(move || {
        let conn = reader_pool.get().expect("取读连接");
        conn.query_row(
            "SELECT count(*) FROM host_settings",
            rusqlite::params![],
            |r| r.get::<_, i64>(0),
        )
        .expect("读操作被写事务阻塞")
    });
    let seen = handle.join().expect("读线程 panic");
    assert_eq!(seen, 1, "WAL 快照读：应看到写事务提交前的旧值");

    tx.commit().expect("提交");

    let after: i64 = pool
        .get()
        .expect("取连接")
        .query_row(
            "SELECT count(*) FROM host_settings",
            rusqlite::params![],
            |r| r.get(0),
        )
        .expect("提交后读");
    assert_eq!(after, 2);
}
