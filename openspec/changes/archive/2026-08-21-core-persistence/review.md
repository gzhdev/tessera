# 代码审查报告 — core-persistence（未提交工作区变更）

- **审查日期**：2026-08-21
- **审查对象**：工作区未提交变更（本变更的实施内容：tessera-store crate 完整实现——r2d2 连接池、迁移器（含备份与降级保护）、六张表 `001_init.sql`、KV 存储与三重配额、插件/权限/设置/虚拟目录访问层，6 个集成测试文件，约 1500 行）
- **审查基准**：`AGENTS.md`（工作区规范）、`plugin-core-architecture-design.md` v0.3（权威设计书）、本变更的 `proposal.md` / `design.md` / `tasks.md` / `specs/`
- **审查方式**：5 路独立审查（规范符合 / 浅层 bug / 历史上下文 / openspec 验收 / 注释符合），发现 10 个候选问题，逐条独立复核评分（0–100 置信度），≥80 为必须报告阈值

## 结论

**无 ≥80 置信度的必须报告问题。** 共发现 10 个问题：1×75、6×50、2×25，另有 1 项设计书张力备忘。最高 75 分，全部低于阈值。

变更整体质量高：

- **OpenSpec 验收逐条核对通过**：24 个已勾选任务的「验证：」标准、persistence spec 12 个 Scenario、storage-quota spec 13 个 Scenario 均有对应实现与真实测试断言（唯任务 1.5 的一个验证点例外，见问题 1）；design.md D1–D7 全部被遵守（rusqlite bundled、r2d2 非自建、include_str! 嵌入 + 单事务含版本记录、计数器同事务、双函数分离保留键、内置别名随迁移种子）
- **AGENTS.md 架构红线全部守住**：rusqlite 走 bundled 特性；错误码 `E_QUOTA_STORAGE` / `E_HOST_DB` 均为既有错误域、三段式构造；网络权限三类型互不隐含（permissions 层纯存取、无推导）；未引入设计书外存储方案
- **浅层 bug 扫描无重大发现**：SQL 全参数绑定（唯一拼接是编译期常量列清单）；写路径 `TransactionBehavior::Immediate`、读-改-写计数器在拿写锁后进行，无 deferred 升级死锁；配额边界全部 `>`（恰好达限允许）且与测试断言一致；迁移状态机正确（降级检查先于一切写入、每条迁移单事务、`VACUUM INTO` 在 autocommit 连接上执行、全新库按「无用户表」而非「文件存在」判定）
- **git 历史无回退**：lib.rs 原为 3 行占位注释（core-workspace-bootstrap 建立），本次纯填充；既有错误域约定（12 域封闭集合）未被扩充；check-deps.sh、schema 一致性检查等 CI 既有步骤不受影响
- 代码注释、文档、用户可见错误信息全部中文，与仓库惯例一致

按 /code-review 约定本次未检查构建信号（fmt 由历史审查路径实测通过，测试归 CI）。

## 接近阈值（75 分）的问题——建议提交前处理

### 1. `rebuild` 无任何开发模式门控，任务 1.5 验收虚勾

- **位置**：`crates/tessera-store/src/migrate.rs:163`（`pub fn rebuild`）；验收标准见 `openspec/changes/core-persistence/tasks.md` 任务 1.5；测试 `crates/tessera-store/tests/migrations.rs:193`（`rebuild_is_a_separate_explicit_path`）
- **问题**（4 个审查代理独立命中）：design.md D4 加粗要求「**仅在开发模式下可用**」，但 `rebuild` 是无条件导出的公开函数——全 crate 无 `#[cfg(debug_assertions)]`、无 feature 门控、无运行时开关（grep 零命中），release 构建下任何调用方均可删库。现有测试本身就是反证：它在非开发模式下成功调用 `rebuild` 并删库，恰好证明该入口可达。任务 1.5 验收标准「该入口在非开发模式下不可达」被勾选 `[x]`，但该验证点客观上未执行——正是 AGENTS.md 明文禁止的「不可只靠编译通过」。
- **「库层无法自判开发模式」的辩护不成立**：`#[cfg(debug_assertions)]` 是编译期事实而非应用层职责，且 cargo test 走 dev profile，门控后现有测试仍可编译运行；退一步，即便门控归属上层，勾选 `[x]` 时该验证点必须在当下为真。
- **修复方向**：给 `rebuild` 加 `#[cfg(debug_assertions)]`（或定义 opt-in feature 由应用在 debug 构建启用），或把 1.5 改回未完成并注明门控归属上层 change。

## 低优先级（50 分）问题——可顺手处理

### 2. `vdirs::upsert` 注释与行为不符：声称不提供写入内置别名的入口，实际可改写

- **位置**：`crates/tessera-store/src/vdirs.rs:19`（注释）与 `:27-32`（实现）
- **问题**：注释称「`builtin` 恒为 0，不提供写入内置别名的入口」，但 `ON CONFLICT (alias) DO UPDATE` 会改写内置别名（workspace/temp/plugin-data）的 `real_path` 与 `writable`（含把本不可写的 workspace/temp 改为可写），且没有 `delete` 中那样的 `builtin = 1` 拒绝检查，防护不对称。当前无生产调用方（仅测试用自建别名）、spec 也只约束删除保护，故未升格；但按设计书 §7.2 内置别名是权限模型锚点，属前瞻性防御缺口。
- **修复方向**：冲突分支拒绝 builtin 行（返回 `Rejected`/`InvalidArgument`），或修正注释。

### 3. `clear_plugin_storage` 违反 design.md D5 同事务纪律

- **位置**：`crates/tessera-store/src/storage.rs:284-292`
- **问题**：DELETE 与 `recalc_on` 分属两个隐式事务，两步之间进程崩溃会留下「数据已删、计数虚高」的配额虚耗，且无自愈路径（`recalc_usage` 无生产调用方）。同文件 `set_plugin_key` / `delete_plugin_key` 均遵守 Immediate 同事务纪律，design.md D5 及 Risks 表明文承诺「计数更新与数据写入同事务」。触发窗口极窄（微秒-毫秒级）且后果为配额误拒而非数据损坏，故 50 分。
- **修复方向**：`recalc_on` 收 `&Connection`，外面包一个事务传 `&tx` 即可。

### 4. 全新库不备份与 persistence spec 字面冲突

- **位置**：`crates/tessera-store/src/migrate.rs:88`；spec 见 `specs/persistence/spec.md`「迁移前自动备份」Requirement 与 Scenario
- **问题**：spec 字面「首次应用任何迁移之前，系统 SHALL 自动备份当前数据库文件」、Scenario WHEN「存在未应用的迁移」均无 fresh-DB 例外，而实现对全新库（无用户表）跳过备份，测试还反向断言「全新建库不应产生空备份」。实现工程上更优（空库备份无意义、还保守覆盖了 schema_migrations 缺失但有其他表的异常库），分歧仅在 spec 措辞层。
- **修复方向**：给 spec 补一句 fresh-DB carve-out（或 Scenario WHEN 加「且数据库非全新」），不改代码。

### 5. 「插件停用时清除 `__picked/*`」未与停用动作接线

- **位置**：`crates/tessera-store/src/plugins.rs:94`（`set_enabled` 不触发清除）与 `crates/tessera-store/src/storage.rs:246`（`clear_picked_entries` 为独立手动入口）；测试 `tests/quota.rs:268-270` 靠先后两次调用模拟
- **问题**：storage-quota spec Requirement「插件停用时，其保留命名空间下的全部交付条目 SHALL 被清除」在本 change 交付后系统层面悬空。proposal 范围边界可辩护（只做存储层、写入时机归 core-host-storage），测试注释也表明是明确的分层决策；但范围边界未显式分配「停用清除」的触发责任，若 core-plugin-lifecycle 立项时遗漏接线，该 Requirement 落空。
- **修复方向**：在 core-plugin-lifecycle 提案中显式承接清除接线；或本 change 的 proposal 范围边界补一句责任归属。

### 6. migrate 模块文档与字段文档的备份触发条件措辞不准

- **位置**：`crates/tessera-store/src/migrate.rs:6`（模块文档「数据库文件已存在」）与 `:44`（字段文档「无文件可备份」），对照 `:88` 实现（按用户表判定）与 `:85-87` 函数内注释（准确）
- **问题**：模块级说「文件已存在→备份」、字段级说「无文件可备份」，与实现判定 `!is_fresh_database`（按用户表）不一致；执行时文件必然已被 SQLite 创建，「无文件可备份」字面不成立。函数内注释与测试注释都是准确的，属概要文档未同步。零功能影响。
- **修复方向**：两处措辞统一为「非全新数据库（存在用户表）」。

### 7. `vdirs::delete` 两连接 TOCTOU 且不检查受影响行数

- **位置**：`crates/tessera-store/src/vdirs.rs:89-102`
- **问题**：builtin 检查与 DELETE 用两个池连接，且 DELETE 后不查 rows_affected——并发下第二个删除者得到 `Ok(())` 但实际未删。是 crate 内唯一不查行数的写路径（`uninstall`/`set_enabled` 等均查并返回 `NotFound`）。现有代码无翻转 builtin 的路径、并发触发方不存在，风险理论性。
- **修复方向**：单条 `DELETE ... WHERE alias = ?1 AND builtin = 0` 加行数检查，原子完成。

### 8. `rebuild` 用 `path.display()` 有损拼接旁文件路径

- **位置**：`crates/tessera-store/src/migrate.rs:164-165`
- **问题**：`format!("{}{suffix}", path.display())` 对非 UTF-8 路径有损转换（变 U+FFFD），删错目标且 `NotFound` 被静默忽略；`suffix = ""` 时主库也用有损路径删除，非 UTF-8 路径下 rebuild 整体静默失效（旧数据原样保留，变成只跑一遍 `run`）。同文件 `backup_path`（`:152-156`）有 `as_os_str().to_os_string().push()` 的无损写法可照抄。触发需开发模式显式调用 + 病态路径（Windows 上仅未配对代理项），实践命中率趋近于零。
- **修复方向**：照抄 `backup_path` 写法，两行改动。

## 25 分（留档）

### 9. `plugin_config` 外键 CASCADE 与设计书 §11.4「卸载保留」语义冲突

- **位置**：`crates/tessera-store/migrations/001_init.sql:53`（外键）；`crates/tessera-store/src/plugins.rs:119-121`（uninstall 级联全删 + 知情注释）
- **问题**：设计书 §11.4 规定 plugin_config 卸载时「保留（重装可恢复设置）」，CASCADE 结构下物理删除。但冲突源头是设计书自身矛盾（§11.2 SQL 草案就写 CASCADE），本 change 的 spec 与 tasks 2.2 逐字要求 CASCADE，实现忠实遵循且在代码注释中显式声明了矛盾与分层论证（保留语义属上层，可先备份再删登记行）。001 未提交、无已发布数据库，修正路径清晰。**备忘：归档本 change 同步设计书时应裁决 §11.2/§11.4 一致性**（spec 内部 :37「值是否保留由用户选择」与 :41「MUST 级联清除」同样互斥，需一并理顺）。

### 10. 生产依赖裸版本号，绕过 workspace.dependencies 惯例

- **位置**：`crates/tessera-store/Cargo.toml:9-11`
- **问题**：`r2d2` / `r2d2_sqlite` / `rusqlite` 直接写裸版本号，是首个破例的内核 crate（tessera-error/manifest/core 的生产依赖一律 `{ workspace = true }`）。AGENTS.md 无明文规定，纯惯例一致性问题，无功能影响。
- **修复方向**：三个依赖入根 `Cargo.toml` `[workspace.dependencies]` 后改 `{ workspace = true }`，顺手可做。
