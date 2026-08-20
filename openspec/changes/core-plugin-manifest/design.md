# core-plugin-manifest · 设计

## Context

见 `proposal.md` 的 Why。要求见 `specs/plugin-manifest/spec.md` 与 `specs/plugin-dependency-graph/spec.md`。

关键约束是 §17.1 的依赖方向：`tessera-manifest` 位于依赖图最底层，不能反向依赖 `tessera-core`。而 §4.4 校验流水线的第 7/8 步（全局 id 冲突）与第 11/12 步的编排都需要跨插件的全局视图。这个张力决定了本 crate 的接口形状。

## Goals / Non-Goals

**Goals**

- 让清单校验的错误信息精确到「哪个字段、错在哪」，这是插件作者接触到的第一个界面
- 让依赖图算法可以脱离全局状态被单元测试
- 让 JSON Schema 不再是需要人维护的第二份真值

**Non-Goals**

- 不持有任何全局注册表。跨插件的冲突检查由 `core-plugin-lifecycle` 编排
- 不做插件目录扫描（那是生命周期的职责）。本 crate 的输入是「一份清单内容 + 它所在的目录路径」

## Decisions

### D1 · 校验分成「纯函数层」与「需要上下文层」，后者用入参而非回调表达

**选择**：把 §4.4 的 12 步切成两组——

```
纯函数（本 crate 独立完成）：
  1 解析  2 结构与未知字段  3 manifestVersion  4 engines.core
  5 engines.pluginApi  6 main 路径  9 activationEvents 悬空引用

需要外部信息（本 crate 提供函数，调用方传入上下文）：
  7  id 冲突          ← 传入「已占用的插件 id 集合」
  8  贡献项冲突        ← 传入「已占用的全限定 id 集合」
  10 虚拟目录别名存在   ← 传入「已注册的别名集合」
  11/12 依赖存在性与环  ← 传入「全部候选清单的集合」
```

后者接收**不可变的集合快照**作为参数，而不是接收一个 trait object 回调。

**理由**：传集合让这些函数保持纯函数性质，可以直接用字面量构造测试用例，不需要 mock。传回调则会把调用方的生命周期与借用规则渗进本 crate 的签名。代价是调用方要先把注册表快照出来——但 §4.4 第 7–8 步本来就要求「在全局写锁下一次性完成」，快照是自然的实现方式。

**放弃的选项**：定义 `trait RegistryView` 由 `tessera-core` 实现（引入抽象但没换来解耦，因为集合语义已经足够简单）。

### D2 · 步骤之间快速失败，步骤内部尽量累积

**选择**：12 个步骤按序执行，任一步失败即停止；但**单个步骤内部**尽可能一次报出全部问题。

**理由**：§4.4 定的是步骤间的顺序语义（后面的步骤依赖前面的结果，比如没解析成功就无法检查字段）。但在同一步里，「你有 5 个字段写错了」一次说完，比让插件作者改一个跑一次改一个跑一次强得多。这直接服务 spec 中「拼写错误可以被直接定位」的要求。

**代价**：serde 的默认行为是遇到第一个错误就停。要累积错误需要额外工作（见 D3 的退路部分）。**若实现成本过高，接受降级为「每步只报第一个错误」**——这不违反任何 spec 要求，只是体验打折。

### D3 · 校验真值在 serde，schema 由 schemars 生成且仅服务补全

**选择**：清单类型标注 `#[serde(deny_unknown_fields)]` 与 `#[derive(JsonSchema)]`，权限用 `#[serde(tag = "type")]` 内部标记枚举。生成的 `plugin.schema.json` check-in 并由 CI 校验一致性。

**理由**：见设计书 §4.3。核心是消除「手写 schema + 手写 Rust 类型」的必然漂移。

**必须首日验证的风险**：serde 的内部标记枚举在分发到 variant 之前会先把内容缓冲进中间表示，历史上与 `deny_unknown_fields` 组合时未知字段检查可能不生效。

**验证方式**：`tessera-manifest` 的**第一个测试**就是 `{"type": "fs.read", "virtualDirectory": "workspace"}` 必须报错。

**退路（按优先级）**：
1. 给每个 variant 的载荷抽成独立 struct 并单独标注 `deny_unknown_fields`
2. 改用邻接标记（`#[serde(tag = "type", content = "scope")]`）——但这会改变清单的 JSON 形状，需要回到设计书 §4.1 改示例，成本高
3. 手写 `Deserialize` 实现——最后手段

### D4 · `engines.core` 用整数比较，不走 SemVer

**选择**：`engines.core: u32`，校验就是 `manifest.engines.core <= CORE_API_LEVEL`。

**理由**：设计书 §13.1（v0.3 决策 A5）已经把这条线从应用 SemVer 改为整数能力级别。用 `semver` crate 处理一个整数是没必要的间接层，且会让「不允许写上界」这条约束难以表达——SemVer 区间天然支持上界。

`engines.pluginApi` 仍然是 SemVer 区间，走 `semver::VersionReq`。

### D5 · 依赖图自己实现 Tarjan，不引入图库

**选择**：手写 Tarjan 强连通分量 + Kahn 拓扑排序，不依赖 `petgraph`。

**理由**：需要的算法只有两个，且都需要**定制输出**——spec 要求环检测的错误信息包含「完整的环路径」，而通用图库返回的是 SCC 的节点集合，还要自己从中还原出一条环路径。自己实现反而更短。图的规模是插件数量（几十到几百），性能无关紧要。

**放弃的选项**：`petgraph`（为两个算法引入一个大依赖，且还要写还原环路径的代码）。

### D6 · 局部名与全限定名用不同类型区分

**选择**：`LocalName` 与 `QualifiedId` 是两个不同的类型，拼接是显式的 `plugin_id.qualify(local)`。

**理由**：§4.2 规定 `contributes` 内写局部名、`activationEvents` 引用本插件时可写局部名、引用他人时必须写全限定名。这是一处极易混淆的地方——用同一个 `String` 表示两者，早晚会出现「把局部名当全限定名注册进去」的 bug，而这类 bug 的表现是插件的命令莫名其妙找不到，很难查。让类型系统挡住。

## Risks / Trade-offs

| 风险 | 缓解 |
|---|---|
| serde 内部标记枚举与 `deny_unknown_fields` 不兼容（D3） | 首个测试即验证；三条退路已排序 |
| 错误累积（D2）实现成本超预期 | 明确接受降级为每步一个错误，不阻塞交付 |
| 生成的 schema 对编辑器补全的实际体验不佳（schemars 对标记枚举生成 `oneOf`） | 补全体验是次要目标——校验真值在 serde 侧。若确实很差，可在生成后加一个后处理步骤，但不列入本 change |
| 「传集合快照」的接口（D1）在插件数量很大时产生可观的拷贝开销 | 桌面场景插件数在几十到几百量级，且校验只在发现与热重载时发生，不在热路径上。接受 |

## Migration Plan

不适用——这是新增能力，没有既有实现需要迁移。

fixture 的组织方式建议先定：`tests/fixtures/valid/` 放正例（首先放入设计书 §4.1 的完整示例），`tests/fixtures/invalid/{错误码}/` 按错误码分目录放反例。这样「每个错误码至少一条反例」这条验收可以用目录遍历自动检查，而不是靠人核对。

## Open Questions

- 校验产生的**警告**（如 `activationEvents` 含 `*`、重复 id）如何回传给调用方——是放在成功结果里，还是走独立的诊断通道。两种都能满足 spec，且不影响任务拆分。倾向放在成功结果里（`Validated { manifest, warnings }`），实现时定。
