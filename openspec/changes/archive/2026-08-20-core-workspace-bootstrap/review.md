# 代码审查报告 — core-workspace-bootstrap（未提交工作区变更）

- **审查日期**：2026-08-20
- **审查对象**：工作区未提交变更（本变更的全部实施内容：8 个 crate 骨架 + Tauri 壳 + CI + WIT 往返示例 + 文档回填）
- **审查基准**：`AGENTS.md`（工作区规范）、`plugin-core-architecture-design.md` v0.3（权威设计书）、本变更的 `proposal.md` / `design.md` / `tasks.md` / `specs/`
- **审查方式**：5 路独立审查（规范符合 / 浅层 bug / 历史上下文 / openspec 验收 / 注释符合），发现的问题经逐条复核评分（0–100 置信度），≥80 为必须报告阈值

## 结论

**无 ≥80 置信度的必须报告问题。** 共发现 6 个独立问题，全部低于 80 阈值。

变更整体质量高：

- crate 依赖方向与设计书 §17.1 完全一致（`sandbox → manifest, ui-schema`；`core → manifest, store, sandbox`；`src-tauri → core`；`tessera-error` 作为新叶子不破坏方向图）
- 错误模型 12 域封闭集合与 §12.1 吻合；可观测性数值（7 天保留 / 8KB 截断 / 100 条每秒每插件）逐项符合 §12.2
- WIT 版本纪律遵守：冒烟 WIT 用独立包名 `tessera:roundtrip@0.1.0`，未触碰 `tessera:plugin@0.2.0`
- `tasks.md` 未勾项（3.3 远程 CI 验证）标注诚实
- 实测验证：`cargo test --workspace` 13 个测试全绿；`scripts/check-deps.sh` 与 `scripts/check-log-no-network.sh` PASS；`cargo build -p tessera-sdk --target wasm32-wasip2` 通过；WIT 往返冒烟 2 个用例通过（`StoreLimits::instances` 实测最小值 3，与 `docs/toolchain-baseline.md` 一致）

## 接近阈值（75 分）的问题——建议提交前处理

以下 4 个问题证据链完整、均经复核确认为真，虽未达报告阈值，但修复成本低、与下游变更直接相关。

### 1. `docs/toolchain-baseline.md` 被 `.gitignore` 屏蔽，任务 1.6 交付物永远不会入库

- **位置**：`.gitignore:1`（`docs/` 规则）
- **问题**：`git check-ignore` 确认命中；`git status` 中该文件连 `??` 都不显示。而它是 tasks.md 1.6（已勾选）的产出，被 design.md 风险表两处、proposal 引用为下游 `core-wasm-sandbox` 的强制基线（含 `StoreLimits::instances` 实测值 3）。干净 checkout 上这些引用全部失效，任务 1.6 的验收（按文档重跑工具链）无法复现。此外 `git ls-files docs/` 为空，docs/ 下任何文件（含设计书历史存档）从未入库。
- **修复**：`.gitignore` 移除 `docs/` 规则或收窄为特定子路径。

### 2. 根 `Cargo.lock` 被忽略，与 build-constraints spec 的可复现承诺直接冲突

- **位置**：`.gitignore:15`（其上方第 13 行注释写明 "Remove Cargo.lock from gitignore if creating an executable"）
- **问题**：本变更已把仓库改造成含 `src-tauri` 二进制的应用 workspace，根 `Cargo.lock`（114KB）与 `examples/toolchain-roundtrip/Cargo.lock`（64KB）均已生成但未被跟踪。`specs/build-constraints/spec.md` 要求「工具链版本组合必须被锁定且可复现」，但 `wasmtime = "47"` 是 semver 区间，不提交锁文件时干净机器会解析到未经实测的 47.x 新版，`docs/toolchain-baseline.md` 记录的实测版本（如 wasmtime 47.0.3）无法复现。
- **修复**：删除 `.gitignore:15` 并提交两份锁文件。

### 3. `scripts/check-log-no-network.sh` 安全检查 fail-open

- **位置**：`scripts/check-log-no-network.sh:14-17`
- **问题**：`out="$(cargo tree ... 2>&1 | grep -vE ...)"` 不检查 cargo tree 退出码。cargo tree 失败时（依赖解析错误等），错误输出被过滤后不含被禁包名，`hits` 为空 → 脚本输出 PASS，安全检查形同虚设。该脚本在 CI（`.github/workflows/ci.yml`）中必跑。同目录 `check-deps.sh` 是 fail-closed 的（cargo 报错且不含 "did not match any packages" 时判 FAIL），两脚本行为不一致佐证这是遗漏。
- **修复**：对 cargo tree 退出码显式判断（如 `|| exit 1` 或捕获后检查 `$?`）。

### 4. MANIFEST 域缺 `E_MANIFEST_VERSION` / `E_MANIFEST_MAIN` 两个已知错误码

- **位置**：`crates/tessera-error/src/codes/manifest.rs:4-6`（仅 `Parse` / `Schema` / `DanglingRef`）；断言见 `crates/tessera-error/src/lib.rs:299-300`
- **问题**：tasks.md 4.1（已勾选）要求「覆盖设计书 §12.1 的全部 12 个域及其已知成员」。§12.1 表格「示例」列只列 3 个 MANIFEST 成员，但设计书正文 §4.4 校验流水线第 3、6 步（`plugin-core-architecture-design.md:537,540`）与 §13.1 错误呈现示例（:1582）明确使用 `E_MANIFEST_VERSION` 与 `E_MANIFEST_MAIN`。`lib.rs` 测试断言 `all.len() >= 38`（38 = §12.1 示例列之和）恰好漏掉这 2 个。下游已立项变更 `core-plugin-manifest` 的 tasks 3.1/3.2 与 spec.md 直接要求返回这两个码，缺失会让下游被迫回头改 tessera-error。
- **修复**：`ManifestCode` 补 `Version` / `Main` 两个变体，断言改为 `>= 40`。

## 低优先级（50 分）问题——可顺手处理

### 5. 限速警告行的 JSON 转义在非 ASCII 输入下非法

- **位置**：`crates/tessera-core/src/observability/limits.rs:132`
- **问题**：手拼 JSON 警告行中 `plugin_id.escape_default()` 对非 ASCII 字符产出 `\u{4e2d}` 这种带花括号的 Rust 风格转义，非法 JSON（JSON 只认 `\uXXXX`）。但设计书 :459 已限定插件 id 为 ASCII 反向域名格式（`^[a-z0-9]+(\.[a-z0-9-]+)+$`），实践中不会触发；且当前无在册代码向该路径喂值。属防御性加固建议：改用 `serde_json::to_string(plugin_id)`。

### 6. design.md 开放问题段与新事实自相矛盾

- **位置**：`openspec/changes/core-workspace-bootstrap/design.md:93`
- **问题**：「CI 平台尚未选定（GitHub Actions / 其他）」——但本次变更新增了 `.github/workflows/ci.yml`（GitHub Actions），平台已选定。本次对 design.md 的其他段落均做了实施注记（D3/D4、风险表、Migration Plan），唯独此处漏改。不影响功能，归档前可顺手更新。

## 修复落地记录（2026-08-20）

全部 6 个问题已在归档前修复完毕，分布于两笔提交：

| # | 问题 | 处置 | 落地提交 |
|---|---|---|---|
| 1 | `docs/` 被 `.gitignore` 屏蔽 | 删除 `docs/` 规则，`docs/` 全部 4 个文件（含 `toolchain-baseline.md`）入库 | 420653b |
| 2 | `Cargo.lock` 被忽略 | 删除 `Cargo.lock` 规则，根锁文件与 `examples/toolchain-roundtrip/Cargo.lock` 均已提交 | 420653b |
| 3 | `check-log-no-network.sh` fail-open | 显式检查 `cargo tree` 退出码，失败即 FAIL；红绿双向验证通过（注入坏包名 → FAIL exit=1，还原 → PASS） | afab754 |
| 4 | MANIFEST 域缺 2 个错误码 | 补 `E_MANIFEST_VERSION` / `E_MANIFEST_MAIN`（注释标明 §4.4 出处），断言 38→40，`cargo test -p tessera-error` 全绿 | afab754 |
| 5 | 丢弃警告行 JSON 转义非法 | 改用 `serde_json::to_string`，`serde_json` 提升为 tessera-core 运行时依赖，7 个 observability 测试全过 | afab754 |
| 6 | design.md:93 开放问题过期 | 划线并加实施注记（已选定 GitHub Actions） | afab754 |

修复后全量门禁复核：`cargo fmt --check`、`clippy -D warnings`、`cargo test --workspace`、`check-deps.sh`、`check-log-no-network.sh`、`openspec validate` 全部通过。
