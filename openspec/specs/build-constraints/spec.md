# build-constraints Specification

## Purpose

规定模块之间不允许存在的依赖方向，以及生成物与其源类型之间必须保持的一致性，使这些架构约束由构建过程强制执行，而不依赖开发者记忆或代码评审的自觉。

## Requirements

### Requirement: 内核模块不得依赖桌面应用框架

`tessera-manifest`、`tessera-ui-schema`、`tessera-store`、`tessera-sandbox`、`tessera-core` 这五个模块的**传递依赖树**中 MUST NOT 出现 Tauri。

桌面框架只允许出现在外壳层，使内核可以在没有窗口的环境（命令行工具、集成测试、未来的其他外壳）中原样复用。

#### Scenario: 违规依赖被构建阻断

- **WHEN** 任何一个内核模块直接或间接引入了 Tauri
- **THEN** 持续集成失败，并指出是哪个模块通过哪条路径引入的

#### Scenario: 外壳层可正常依赖框架

- **WHEN** 外壳层引入 Tauri
- **THEN** 检查通过

### Requirement: 内核编排层不得依赖沙箱运行时

`tessera-core` 的**传递依赖树**中 MUST NOT 出现 wasmtime。

所有沙箱运行时类型 MUST 被封装在 `tessera-sandbox` 之内，内核只通过与运行时无关的抽象与之交互。这条约束是「将来把某类插件挪到独立进程而不改动内核」这一演进路径的前提。

#### Scenario: 违规依赖被构建阻断

- **WHEN** `tessera-core` 直接或间接引入了 wasmtime
- **THEN** 持续集成失败，并指出引入路径

#### Scenario: 沙箱层可正常依赖运行时

- **WHEN** `tessera-sandbox` 引入 wasmtime
- **THEN** 检查通过

### Requirement: 跨语言契约描述必须由源类型生成且保持一致

面向其他语言或工具的契约描述文件 MUST 由权威的源类型自动生成，MUST NOT 手工编写或手工修改。

生成结果 SHALL 纳入版本控制以便评审契约变更；持续集成 MUST 校验「重新生成后的内容与已提交内容逐字节一致」。

#### Scenario: 手改生成物被发现

- **WHEN** 有人直接编辑了生成的契约描述文件而没有改源类型
- **THEN** 持续集成失败，因为重新生成的结果与已提交内容不一致

#### Scenario: 改了源类型但忘记重新生成

- **WHEN** 源类型被修改，但生成物未同步更新并提交
- **THEN** 持续集成失败，提示需要重新生成

### Requirement: 工具链版本组合必须被锁定且可复现

系统 SHALL 记录一份经过验证的工具链版本组合，覆盖构建 WASM 组件所需的全部环节。

持续集成 SHALL 包含一个最小的组件往返用例，用于在工具链升级时立即暴露不兼容。

#### Scenario: 工具链漂移被立即发现

- **WHEN** 工具链中任一环节升级到不兼容的版本
- **THEN** 往返用例失败，而不是等到实现真实插件时才暴露

#### Scenario: 新环境可复现构建

- **WHEN** 一台全新的开发机按记录的版本组合安装工具链
- **THEN** 往返用例可以成功执行
