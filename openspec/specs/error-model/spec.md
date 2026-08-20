# error-model Specification

## Purpose

规定系统中所有失败的分类方式与呈现方式，使任何一次失败都能被程序判定、被用户理解、被开发者定位，并且这三种用途互不干扰。

## Requirements

### Requirement: 错误必须归属一个域前缀

每个错误 SHALL 携带一个 `E_{DOMAIN}_{REASON}` 形式的错误码，且 `{DOMAIN}` MUST 属于以下封闭集合：`MANIFEST`、`COMPAT`、`DEPS`、`CONFLICT`、`LOAD`、`ACTIVATE`、`PERM`、`QUOTA`、`TIMEOUT`、`TRAP`、`CALL`、`HOST`。

新增错误码 MUST 落在已有域内；引入新域 MUST 作为一次显式的契约变更处理。

#### Scenario: 错误码可被程序判定

- **WHEN** 调用方收到一个失败结果
- **THEN** 该结果携带的错误码可与已知常量做相等比较，无需解析人类可读文本

#### Scenario: 错误码可跨边界传递

- **WHEN** 错误需要从宿主传递给前端或插件
- **THEN** 错误码序列化为字符串后传递，且反序列化回来与原值相等

### Requirement: 错误必须携带三段互不替代的信息

每个错误 SHALL 同时携带三段信息：错误码（供程序判定）、面向用户的一句话（说明发生了什么，不含技术细节）、面向开发者的细节（完整路径、回溯、相关 id）。

面向开发者的细节 MUST NOT 出现在用户界面中；错误码与面向用户的一句话 SHALL 出现在用户界面中。

#### Scenario: 用户看到可理解的失败原因

- **WHEN** 插件因无权访问某目录而失败
- **THEN** 用户界面显示错误码与一句话原因（如「插件 X 无权访问该目录」）
- **AND** 完整的真实路径与调用回溯只出现在日志中

#### Scenario: 开发者细节可选缺省

- **WHEN** 某个错误没有额外的技术细节可提供
- **THEN** 该错误仍然合法，开发者细节一段为空

### Requirement: 宿主内部错误必须与外部错误区分

`E_HOST_*` 域 SHALL 专用于宿主自身的缺陷（如宿主代码 panic、数据库访问失败）。

产生 `E_HOST_*` 错误时，系统 MUST 在日志中将其标记为宿主 bug，而非插件的问题。

#### Scenario: 宿主缺陷不被误报为插件问题

- **WHEN** 宿主在处理某插件的请求时自身发生 panic
- **THEN** 错误码为 `E_HOST_PANIC`，日志标记为宿主 bug
- **AND** 用户界面不将失败归因于该插件的代码质量
