# core-persistence

## Why

宿主的几乎所有长期状态都落在一个 SQLite 文件里：已安装插件、权限授予记录、虚拟目录映射、插件配置值、插件私有 KV、窗口与面板布局（§11.2）。这六张表被后续至少 8 个 change 消费，它们的 schema 与访问方式必须先定下来，否则每个 change 都会自己拍一套表结构。

迁移策略尤其不能后补。§11.3 定的是「只前滚，不回滚；`db_version >` 代码支持版本时**拒绝启动**」——这条规则要在第一次建库时就生效，否则等到有真实用户数据后再引入迁移框架，第一次迁移本身就没有安全网。

这块同样**完全不依赖 wasmtime**，可与工具链验证、清单校验并行开工，且能用临时数据库文件独立验收。

## What Changes

- §11.2 的完整 DDL：`schema_migrations` / `plugins` / `plugin_permissions` / `virtual_dirs` / `plugin_config` / `plugin_storage` / `host_settings`，含 WAL、外键、`synchronous = NORMAL`
- 迁移器：脚本按 `{NNN}_{name}.sql` 编号存放并用 `include_str!` 嵌入二进制；启动时按序执行未应用脚本，每个脚本在单事务内完成；首次迁移前自动备份为 `tessera.db.bak-{version}`
- **降级保护**：检测到 `db_version >` 代码支持的最大版本时拒绝启动并给出明确提示，而非用旧代码操作新 schema
- 连接池：worker 线程会并发访问，WAL 下多读一写不互斥；写操作短小，不做长事务
- 六张表的 typed 访问层，各自封住 SQL，上层不写裸查询
- 配额计数（§11.5）：`plugin_storage` 的 16 MB / 10000 键 / 单值 1 MB；`__` 保留前缀的写入拒绝；`__picked/*` 的配额豁免与 `MAX_PICKED_ENTRIES` 淘汰
- 内置虚拟目录别名（`plugin-data` / `workspace` / `temp`）的初始化与 `builtin` 标记保护

**范围边界**：本 change 只做**存储层**。配置项的 schema 校验、前缀强制、`core/config-changed` 事件属于 `core-host-config`；`__picked/*` 的写入时机与 token 生成属于 `core-host-storage`。这里只保证「表存在、约束正确、读写可用、配额可查」。另一项显式留给下游的责任：**插件停用时清除 `__picked/*` 的触发接线**归 `core-plugin-lifecycle`——存储层只提供 `clear_picked_entries` 能力，spec storage-quota 的该条 Requirement 由承接方落地（review.md 问题 5）。

## Capabilities

### New Capabilities

- `persistence`：本地数据库的 schema、迁移与降级保护、连接管理，以及各表的读写契约
- `storage-quota`：插件私有存储的配额规则、保留命名空间，以及超限与豁免的判定

### Modified Capabilities

（无）

## Impact

**新增**

- `crates/tessera-store/src/{pool.rs, migrate.rs, plugins.rs, permissions.rs, vdirs.rs, config.rs, storage.rs, settings.rs}`
- `crates/tessera-store/migrations/001_init.sql`
- `crates/tessera-store/tests/`：迁移幂等、降级拒绝、配额边界、保留前缀拒写

**新增依赖**

- `rusqlite`（`bundled` 特性，避免依赖系统 SQLite）
- 连接池：`r2d2` + `r2d2_sqlite`，或自建 `Mutex<Vec<Connection>>`（见 design.md 的取舍）

**下游影响**

- `core-plugin-lifecycle` 读写 `plugins` 表记录安装与激活时间
- `core-permission-model` 读 `plugin_permissions` 与 `virtual_dirs` 构建权限快照
- `core-host-config` / `core-host-storage` 建立在 `plugin_config` / `plugin_storage` 之上
- `ui-panel-system` 把布局持久化进 `host_settings`

**风险**

- 迁移只前滚的策略意味着一旦 `001_init.sql` 发布出去就不能再改。开发期需要一条明确的「重建数据库」路径，且必须与正式迁移路径区分开，否则会有人在开发期改 001 而线上库已经应用过旧版本
