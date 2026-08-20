## Purpose

定义插件之间依赖关系的解析规则，据此推导出正确的加载顺序、停用顺序与失败传播范围，并在依赖关系不可满足时给出可定位的错误。

## ADDED Requirements

### Requirement: 依赖必须存在且版本满足

依赖图 SHALL 以插件为节点、`dependencies` 声明为有向边构建。

依赖的插件标识不存在时返回 `E_DEPS_MISSING`；存在但版本不落在声明的 SemVer 区间内时返回 `E_DEPS_VERSION`。两种情况下发起依赖的插件均进入失败状态。

插件 MUST NOT 依赖自身。

#### Scenario: 依赖缺失

- **WHEN** 插件 A 声明依赖 `com.example.b`，但该插件未安装
- **THEN** A 进入失败状态并返回 `E_DEPS_MISSING`，错误信息指出缺失的是哪个插件

#### Scenario: 版本不满足

- **WHEN** 插件 A 声明依赖 `com.example.b` 的 `^2.0.0`，而已安装的 B 是 `1.5.0`
- **THEN** A 进入失败状态并返回 `E_DEPS_VERSION`，错误信息同时给出要求区间与实际版本

#### Scenario: 自依赖被拒绝

- **WHEN** 插件在 `dependencies` 中声明了自己的标识
- **THEN** 校验失败

### Requirement: 依赖环必须被检出且完整报告

依赖图中存在环时，环上的**所有**插件 SHALL 进入失败状态并返回 `E_DEPS_CYCLE`。

错误信息 MUST 包含完整的环路径，而不仅仅是「存在环」。自环同样视为环。

#### Scenario: 三插件环被完整报告

- **WHEN** A 依赖 B、B 依赖 C、C 依赖 A
- **THEN** A、B、C 全部进入失败状态并返回 `E_DEPS_CYCLE`
- **AND** 错误信息中含形如 `A → B → C → A` 的完整路径

#### Scenario: 自环被检出

- **WHEN** 某插件的依赖链最终指回自己
- **THEN** 返回 `E_DEPS_CYCLE`

#### Scenario: 环外插件不受影响

- **WHEN** 图中既有一个环，也有不参与该环的插件
- **THEN** 只有环上的插件进入失败状态，其余插件照常处理

### Requirement: 加载顺序必须满足拓扑序

依赖图无环时，系统 SHALL 推导出一个加载顺序，使任何插件被激活之前，它的全部直接与间接依赖都已激活。

停用顺序 SHALL 是加载顺序的逆序：停用某插件之前，所有依赖它的插件都已先行停用。

#### Scenario: 依赖先于被依赖方激活

- **WHEN** A 依赖 B，两者都需要激活
- **THEN** B 的激活在 A 之前完成

#### Scenario: 停用顺序为逆拓扑序

- **WHEN** A 依赖 B，两者都在运行且需要全部停用
- **THEN** A 先于 B 停用

### Requirement: 依赖失败必须级联且只报根因

某插件激活失败时，所有直接或间接依赖它的插件 SHALL 进入失败状态并返回 `E_DEPS_UPSTREAM_FAILED`。

级联失败的错误信息 MUST 指明根因插件的标识。用户可见的失败呈现 MUST 只针对根因报告一次，被级联的插件标记为「因依赖 X 失败」并链接到根因，MUST NOT 为每个被波及的插件各产生一次通知。

#### Scenario: 级联失败指向根因

- **WHEN** B 激活失败，而 A 依赖 B、C 依赖 A
- **THEN** A 与 C 都进入失败状态并返回 `E_DEPS_UPSTREAM_FAILED`
- **AND** 两者的错误信息都指向根因插件 B

#### Scenario: 一次失败只通知一次

- **WHEN** 一个根因导致 5 个插件级联失败
- **THEN** 用户只收到一条关于根因的通知，而不是 6 条

### Requirement: 声明依赖不等同于获得调用权

`dependencies` 的作用范围 SHALL 限于加载顺序与存在性保证。

插件调用另一插件的命令 MUST 另行持有对应的调用权限；仅声明依赖 MUST NOT 隐含任何调用能力。

#### Scenario: 有依赖但无调用权限

- **WHEN** A 声明依赖 B，但未声明对 B 命令的调用权限
- **THEN** B 会先于 A 被激活，但 A 调用 B 的命令时被拒绝
