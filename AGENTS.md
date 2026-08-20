# AGENTS.md — Tessera 工作区须知

## 项目概况

Tessera：桌面插件化工具核心。内核保持薄、稳定、通用，业务功能全部通过插件契约挂载。**当前处于设计阶段**——架构已定稿（v0.3），OpenSpec 变更提案已就绪，但 `src/` 仍是 hello-world 占位，`Cargo.toml` 尚无依赖。

技术栈（设计书 §1.2，不可随意更换）：

- Tauri 2.x 桌面壳 + Rust edition 2024 核心 + Vue 3.4+ / TS 5.x 前端
- wasmtime（WASM Component Model + WASI 0.2）做进程内插件沙箱；插件接口用 WIT + wit-bindgen，编译目标 `wasm32-wasip2`
- SQLite（`rusqlite` bundled 特性）本地存储

## 目录与文档权威性

- `plugin-core-architecture-design.md`（仓库根）— **权威架构设计书 v0.3**。改动涉及契约细节前必读相关章节。
- `docs/` — 旧版存档（v0.1 / v0.2）及当前版副本；历史版本不要编辑。
- `openspec/changes/<change-id>/` — 每个变更含 `proposal.md`、`design.md`、`tasks.md`、`specs/*/spec.md`。
- `src/` — Rust crate `tessera`（尚未开工）。

## OpenSpec 工作流（本仓库强制）

- 所有实质性变更走 OpenSpec：提出用 `openspec-propose` 技能，实施用 `openspec-apply-change`，修订用 `openspec-update-change`，归档用 `openspec-archive-change`。不要绕过提案直接改代码。
- `openspec` CLI 已全局安装（npm）。schema 为 `spec-driven`（见 `openspec/config.yaml`）。
- 已有五个待实施变更：`core-persistence`、`core-plugin-manifest`、`core-workspace-bootstrap`、`ui-component-library`、`ui-schema-and-typegen`。
- `tasks.md` 中每个任务带「验证：」验收标准——实现后必须逐条实际验证，不可只靠编译通过。

## 常用命令

```bash
cargo check          # 尚无 src 代码，当前仅占位
cargo test
openspec list        # 查看变更状态
```

## 架构红线（设计书 v0.3 已定死，实现时不得推翻）

- 半可信信任模型：沙箱目标是故障隔离与资源限额，不防御蓄意恶意插件。
- 进程内 WASM 沙箱、Component Model + WIT、每插件一线程、全同步阻塞、声明式 UI 描述树。
- 网络权限是三种独立类型 `net.http` / `net.insecure` / `net.private`，互不隐含。
- `engines.core` 是整数能力级别 `CORE_API_LEVEL`，与应用 SemVer 解耦。
- 清单 schema 从 Rust 类型生成（serde tagged enum + `deny_unknown_fields`），`plugin.schema.json` 只是生成物。
- WIT 版本纪律以 M1 为生效起点，此前 `tessera:plugin@0.2.0` 视为未发布。

## 约定

- 文档、提案、提交信息一律用中文；提交信息带前缀，如 `ADD: ...`、`Init: ...`。
- 工作分支 `dev`，PR 目标分支 `master`。
- 不确定某 API 怎么用时，用 `gh_grep` 搜 GitHub 真实代码示例。
- 查库文档用 `context7`（先 resolve-library-id 再 query-docs）。
- 在思考和回复用户时**必须**使用简体中文。
