# core-plugin-manifest · 任务

> 第 1 组必须最先完成。design.md D3 记录的 serde 行为风险决定了整个类型定义的形态——若它不成立，第 2 组的写法要改。

## 1. serde 行为验证（阻塞性）

- [x] 1.1 定义一个最小的内部标记枚举（`#[serde(tag = "type", deny_unknown_fields)]`），带两个 variant。验证：`{"type": "fs.read", "virtualDirectory": "workspace"}` 必须反序列化失败，且错误信息中含 `virtualDirectory`
- [x] 1.2 若 1.1 失败，按 design.md D3 的退路顺序依次尝试（每 variant 载荷独立 struct 单独标注 → 邻接标记 → 手写 Deserialize），选定可行方案。验证：1.1 的用例在选定方案下通过
- [x] 1.3 把结论记入 design.md 的 Decisions（若走了退路，说明实际采用的表示形式）。验证：文档与代码中实际使用的 serde 属性一致

## 2. 清单类型定义

> 对应 `specs/plugin-manifest/spec.md` 的字段规范类 Requirement

- [x] 2.1 定义顶层 `PluginManifest` 及 `Engines`、`Contributes` 等子结构，全部标注 `deny_unknown_fields`。验证：顶层出现未知字段时反序列化失败并返回 `E_MANIFEST_SCHEMA`
- [x] 2.2 定义 `Permission` 枚举，覆盖 8 种权限类型（`fs.read`/`fs.write`/`net.http`/`net.insecure`/`net.private`/`events.publish`/`events.subscribe`/`command.invoke`）。验证：8 种各有一条正例 fixture 解析成功；`shell.execute` 被拒绝
- [x] 2.3 定义 `LocalName` 与 `QualifiedId` 两个不同类型及 `qualify` 拼接函数（design.md D6）。验证：类型系统阻止把局部名直接当全限定名使用；`com.example.myplugin` + `run` 拼出 `com.example.myplugin.run`
- [x] 2.4 定义 `ViewKind` 枚举（当前仅 `Declarative`）并设为省略时的默认值。验证：省略 `kind` 的视图解析成功且值为 `declarative`；写 `"kind": "webview"` 解析失败
- [x] 2.5 定义配置项类型，含 6 种值类型、必填的 `default`、可选的 `sensitive`。验证：缺 `default` 的配置项解析失败；标注 `sensitive: true` 的项解析后该标注可被读取
- [x] 2.6 实现标识符的格式与长度校验（`id` 反向域名且 ≤128、`name` ≤64、`description` ≤256、`version` 严格 SemVer）。验证：每条约束各有一条越界反例返回 `E_MANIFEST_SCHEMA`

## 3. 无上下文的校验步骤

> 对应 design.md D1 的「纯函数」组

- [x] 3.1 实现 `main` 路径校验：必须 `.wasm` 结尾、无 `..` 分量、非绝对路径、无盘符；路径合法但文件不存在返回 `E_MANIFEST_MAIN`。验证：`../../evil.wasm` 返回 `E_MANIFEST_SCHEMA`；合法但缺失的文件返回 `E_MANIFEST_MAIN`
- [x] 3.2 实现 `manifestVersion` 校验（仅接受 1）。验证：`2` 返回 `E_MANIFEST_VERSION`
- [x] 3.3 实现 `engines.core` 整数比较（design.md D4）。验证：插件要 2 而宿主为 1 时返回 `E_COMPAT_CORE`，错误信息含双方数值与建议动作；插件要 1 而宿主为 3 时通过
- [x] 3.4 实现 `engines.pluginApi` 的 SemVer 区间匹配。验证：插件要 `^0.3.0` 而宿主提供 `0.2.0` 时返回 `E_COMPAT_API`
- [x] 3.5 实现 `activationEvents` 的语法校验与本插件内的悬空引用检查。验证：`onCommand:doesNotExist` 返回 `E_MANIFEST_DANGLING_REF`；`*` 通过但产生警告

## 4. 需要外部上下文的校验步骤

> 对应 design.md D1 的「需要外部信息」组，全部以不可变集合作为入参

- [x] 4.1 实现插件 id 冲突检查（入参：已占用的插件 id 集合）。验证：id 已在集合中时返回 `E_CONFLICT_PLUGIN_ID`
- [x] 4.2 实现贡献项 id 冲突检查（入参：已占用的全限定 id 集合）。验证：某贡献项拼接后的全限定 id 已被占用时返回 `E_CONFLICT_CONTRIB_ID`
- [x] 4.3 实现虚拟目录别名存在性检查（入参：已注册别名集合）。验证：声明 `{"type":"fs.read","virtualDir":"nonexistent"}` 时返回 `E_PERM_UNKNOWN_VDIR`
- [x] 4.4 实现校验流水线编排：12 步按序，步骤间快速失败，步骤内尽量累积（design.md D2）。验证：一份同时含多种错误的清单，返回的是**第一个失败步骤**的错误，且该步骤内的多个字段问题一次报出

## 5. 依赖图

> 对应 `specs/plugin-dependency-graph/spec.md`

- [x] 5.1 实现依赖图构建与缺失/版本检查。验证：依赖不存在返回 `E_DEPS_MISSING`；版本不满足返回 `E_DEPS_VERSION` 且错误信息同时含要求区间与实际版本；自依赖被拒绝
- [x] 5.2 实现 Tarjan 强连通分量环检测，并从 SCC 还原出完整环路径（design.md D5）。验证：A→B→C→A 时三者全部返回 `E_DEPS_CYCLE`，错误信息含 `A → B → C → A` 形式的完整路径；自环被检出
- [x] 5.3 验证环外插件不受影响。验证：图中同时存在一个环与若干无关插件时，只有环上插件失败
- [x] 5.4 实现拓扑排序产出加载顺序，及其逆序作为停用顺序。验证：A 依赖 B 时加载序中 B 在 A 前、停用序中 A 在 B 前
- [x] 5.5 实现级联失败传播：标记全部直接与间接依赖失败插件的下游，返回 `E_DEPS_UPSTREAM_FAILED` 并携带根因插件标识。验证：B 失败、A 依赖 B、C 依赖 A 时，A 与 C 的错误都指向根因 B

## 6. schema 生成与 fixture 体系

- [x] 6.1 为清单类型加 `JsonSchema` 派生，实现生成命令产出 `crates/tessera-manifest/schema/plugin.schema.json` 并 check-in。验证：生成命令可重复执行且产出稳定
- [x] 6.2 在 CI 中加入生成物一致性检查。验证：改了类型未重新生成时 CI 失败；手改生成物未改类型时 CI 也失败
- [x] 6.3 建立 fixture 目录结构：`tests/fixtures/valid/` 与 `tests/fixtures/invalid/{错误码}/`。验证：把设计书 §4.1 的完整示例放入 valid/ 后校验通过，零错误零警告
- [x] 6.4 补齐反例 fixture，使每个本 change 涉及的错误码至少有一条。验证：一个遍历式测试自动检查「每个错误码目录下至少一个文件，且每个文件恰好触发该错误码」——不靠人工核对清单
