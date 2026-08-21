-- ============================================================================
-- 001_init.sql —— 初始结构（设计书 §11.2 全部六张表 + schema_migrations）
--
-- !! 此文件已冻结，结构变更请新增迁移（002_*.sql、003_*.sql …）!!
-- 迁移只前滚（§11.3）：已应用过本文件的数据库永远不会重新执行它，
-- 事后修改只会让新装机与老库的结构分叉。开发期需要改结构时，
-- 用重建入口 tessera_store::migrate::rebuild() 删库重来（仅限开发模式）。
--
-- 连接级 PRAGMA（journal_mode = WAL / foreign_keys = ON / synchronous = NORMAL）
-- 在 pool.rs 中随每个连接设置，不放迁移脚本——journal_mode 无法在事务内变更。
-- ============================================================================

-- 迁移版本（迁移器先检测此表是否存在：不存在视为全新数据库，从版本 0 起按序应用）
CREATE TABLE schema_migrations (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL
);

-- 已安装插件。storage_bytes / storage_keys 为配额增量计数器（design.md D5），
-- 与 plugin_storage 的写入在同一事务内更新。
CREATE TABLE plugins (
    plugin_id          TEXT PRIMARY KEY,
    version            TEXT NOT NULL,
    install_path       TEXT NOT NULL,
    manifest_json      TEXT NOT NULL,      -- 原始清单，供审计与升级 diff
    enabled            INTEGER NOT NULL DEFAULT 1,
    installed_at       TEXT NOT NULL,
    last_activated_at  TEXT,
    storage_bytes      INTEGER NOT NULL DEFAULT 0,
    storage_keys       INTEGER NOT NULL DEFAULT 0
);

-- 权限授予记录
CREATE TABLE plugin_permissions (
    plugin_id       TEXT NOT NULL REFERENCES plugins(plugin_id) ON DELETE CASCADE,
    permission_type TEXT NOT NULL,        -- fs.read / net.http / ...
    scope_json      TEXT NOT NULL,        -- {"virtualDir":"workspace"} 等，规范化后的 JSON
    granted         INTEGER NOT NULL DEFAULT 1,
    decided_at      TEXT NOT NULL,
    PRIMARY KEY (plugin_id, permission_type, scope_json)
);

-- 虚拟目录别名映射
CREATE TABLE virtual_dirs (
    alias      TEXT PRIMARY KEY,
    real_path  TEXT NOT NULL,
    writable   INTEGER NOT NULL DEFAULT 0,
    builtin    INTEGER NOT NULL DEFAULT 0  -- 内置别名不可被用户删除
);

-- 插件配置项的值（schema 在 manifest 里，这里只存被改过的值）
CREATE TABLE plugin_config (
    plugin_id  TEXT NOT NULL REFERENCES plugins(plugin_id) ON DELETE CASCADE,
    key        TEXT NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (plugin_id, key)
);

-- 插件私有 KV（插件的业务数据，宿主不解释内容）
CREATE TABLE plugin_storage (
    plugin_id  TEXT NOT NULL REFERENCES plugins(plugin_id) ON DELETE CASCADE,
    key        TEXT NOT NULL,
    value      BLOB NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (plugin_id, key)
);
CREATE INDEX idx_plugin_storage_plugin ON plugin_storage(plugin_id);

-- 宿主自身设置（含窗口布局、面板状态、快捷键覆盖）
CREATE TABLE host_settings (
    key        TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 内置虚拟目录别名（§7.2 / design.md D7）：随迁移插入并标记 builtin，
-- 让「不可删除」成为数据约束而非代码里的黑名单。
-- 真实路径运行期由壳层/权限层填充：plugin-data 与 temp 的路径含 {plugin_id}
-- 模板，workspace 等待用户选定目录——未映射（空串）是合法状态。
-- writable 依据 §7.2 默认权限：plugin-data 自动授予读写；workspace / temp 需声明。
INSERT INTO virtual_dirs (alias, real_path, writable, builtin) VALUES
    ('plugin-data', '', 1, 1),
    ('workspace',   '', 0, 1),
    ('temp',        '', 0, 1);
