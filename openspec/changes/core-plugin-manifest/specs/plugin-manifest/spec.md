## Purpose

定义插件清单的结构与校验规则，使宿主能够在不执行任何插件代码的前提下认识一个插件——它是谁、需要什么权限、贡献了哪些功能、以及它是否与当前宿主兼容。

## ADDED Requirements

### Requirement: 清单必须以固定文件名存在于插件目录

每个插件目录 SHALL 包含一个名为 `plugin.json` 的文件。该文件不存在或不是合法 JSON 时，插件校验 SHALL 失败并返回 `E_MANIFEST_PARSE`。

#### Scenario: 缺少清单

- **WHEN** 扫描到一个不含 `plugin.json` 的目录
- **THEN** 该目录不被识别为插件

#### Scenario: 清单不是合法 JSON

- **WHEN** `plugin.json` 内容有语法错误
- **THEN** 校验失败并返回 `E_MANIFEST_PARSE`，错误信息指出出错位置

### Requirement: 未知字段必须被拒绝而非忽略

清单中出现规范未定义的字段时，校验 SHALL 失败并返回 `E_MANIFEST_SCHEMA`。

错误信息 MUST 指出是哪个上下文中的哪个未知字段，使拼写错误可以被直接定位。

#### Scenario: 字段名拼写错误得到精确反馈

- **WHEN** 权限条目写成 `{"type": "fs.read", "virtualDirectory": "workspace"}`
- **THEN** 校验失败，错误信息指出 `fs.read` 不认识字段 `virtualDirectory`
- **AND** 错误信息不是「未匹配任何子模式」这类无法定位的表述

#### Scenario: 顶层未知字段同样被拒绝

- **WHEN** 清单顶层出现一个规范未定义的字段
- **THEN** 校验失败并返回 `E_MANIFEST_SCHEMA`

### Requirement: 标识符必须符合命名规范且长度受限

`id` MUST 匹配反向域名格式且长度不超过 128；`name` 长度不超过 64；`description` 长度不超过 256；`version` MUST 是严格的 SemVer。

`contributes` 内部的标识符 SHALL 写局部名，由宿主在注册时拼接为 `{plugin_id}.{local_name}` 形式的全限定名。

#### Scenario: 非法 id 被拒绝

- **WHEN** `id` 为 `MyPlugin`（含大写、无点分隔）
- **THEN** 校验失败并返回 `E_MANIFEST_SCHEMA`

#### Scenario: 局部名被拼接为全限定名

- **WHEN** id 为 `com.example.myplugin` 的插件贡献了局部名为 `run` 的命令
- **THEN** 该命令的全限定标识为 `com.example.myplugin.run`

#### Scenario: 引用本插件时可用局部名

- **WHEN** `activationEvents` 中引用本插件的命令
- **THEN** 允许写局部名；引用其他插件的命令时 MUST 写全限定名

### Requirement: 主模块路径必须受限且存在

`main` MUST 以 `.wasm` 结尾，MUST NOT 包含 `..` 分量，MUST NOT 是绝对路径或含盘符。

路径合法但文件不存在时，校验 SHALL 失败并返回 `E_MANIFEST_MAIN`。

#### Scenario: 路径逃逸被拒绝

- **WHEN** `main` 为 `../../evil.wasm`
- **THEN** 校验失败并返回 `E_MANIFEST_SCHEMA`

#### Scenario: 文件缺失被拒绝

- **WHEN** `main` 路径合法但对应文件不存在
- **THEN** 校验失败并返回 `E_MANIFEST_MAIN`

### Requirement: 三条版本线必须分别校验

清单校验 SHALL 分别检查三条独立的版本约束：

- `manifestVersion` 为整数，当前仅接受 `1`，否则返回 `E_MANIFEST_VERSION`
- `engines.core` 为正整数，表示插件要求的最低宿主能力级别；大于宿主当前能力级别时返回 `E_COMPAT_CORE`
- `engines.pluginApi` 为 SemVer 区间，与宿主提供的插件 API 版本不匹配时返回 `E_COMPAT_API`

不兼容时 MUST 拒绝加载，MUST NOT 尝试降级运行。错误信息 MUST 同时包含插件要求的值、宿主当前提供的值、以及建议动作。

#### Scenario: 能力级别不足时给出可操作的错误

- **WHEN** 插件声明 `"core": 2` 而宿主当前能力级别为 `1`
- **THEN** 校验失败并返回 `E_COMPAT_CORE`
- **AND** 错误信息形如「此插件需要宿主能力级别 ≥ 2，当前宿主为 1，请升级应用」

#### Scenario: 能力级别向上兼容

- **WHEN** 插件声明 `"core": 1` 而宿主当前能力级别为 `3`
- **THEN** 校验通过

#### Scenario: 插件 API 不兼容在加载前被拦截

- **WHEN** 插件声明 `"pluginApi": "^0.3.0"` 而宿主提供 `0.2.0`
- **THEN** 校验失败并返回 `E_COMPAT_API`，而不是留给运行时给出底层的导入不匹配错误

### Requirement: 激活事件必须语法合法且引用有效

`activationEvents` MUST 非空。每一项 MUST 匹配已定义的激活事件语法：`onCommand:{id}`、`onView:{id}`、`onEvent:{topic}`、`onStartup`、`*`。

引用了本插件 `contributes` 中不存在的命令或视图时，校验 SHALL 失败并返回 `E_MANIFEST_DANGLING_REF`。

#### Scenario: 悬空引用被拦截

- **WHEN** `activationEvents` 含 `onCommand:doesNotExist` 而 `contributes.commands` 中没有该命令
- **THEN** 校验失败并返回 `E_MANIFEST_DANGLING_REF`

#### Scenario: 无条件激活被允许但标记

- **WHEN** `activationEvents` 含 `*`
- **THEN** 校验通过，但产生一条警告，提示该写法仅供开发调试

### Requirement: 权限声明必须归属已定义的权限类型

`permissions` 中每一项的 `type` MUST 属于已定义的权限类型集合：`fs.read`、`fs.write`、`net.http`、`net.insecure`、`net.private`、`events.publish`、`events.subscribe`、`command.invoke`。

每种类型 MUST 携带且仅携带其规定的 scope 字段。同一 `type` 与 scope 的组合 MUST NOT 重复出现。

引用了宿主未注册的虚拟目录别名时，校验 SHALL 失败并返回 `E_PERM_UNKNOWN_VDIR`。

#### Scenario: 三类网络权限各自独立

- **WHEN** 插件同时声明 `net.http` 与 `net.private`
- **THEN** 校验通过，两条权限各自携带自己的域名列表

#### Scenario: 未知权限类型被拒绝

- **WHEN** 声明了 `{"type": "shell.execute"}`
- **THEN** 校验失败并返回 `E_MANIFEST_SCHEMA`

#### Scenario: 未知虚拟目录别名被拒绝

- **WHEN** 声明了 `{"type": "fs.read", "virtualDir": "nonexistent"}` 而宿主未注册该别名
- **THEN** 校验失败并返回 `E_PERM_UNKNOWN_VDIR`

### Requirement: 视图必须声明其种类且当前只接受声明式

`contributes.views` 中每一项的 `kind` 字段 SHALL 可省略，省略时取默认值 `declarative`。

当前 `kind` MUST NOT 取 `declarative` 以外的值。

#### Scenario: 省略 kind 时取默认值

- **WHEN** 视图条目未写 `kind`
- **THEN** 校验通过，该视图被视为 `declarative`

#### Scenario: 其他种类被拒绝

- **WHEN** 视图条目写 `"kind": "webview"`
- **THEN** 校验失败并返回 `E_MANIFEST_SCHEMA`

### Requirement: 配置项必须带默认值且可标注敏感

`contributes.configuration.properties` 中每一项 MUST 有 `default`，类型 MUST 属于 `string` / `integer` / `number` / `boolean` / `string[]` / `enum`。

每一项 SHALL 可选标注 `sensitive: true`，用于向宿主表明该值需要密码框呈现与日志脱敏。

#### Scenario: 缺少默认值被拒绝

- **WHEN** 某配置项未声明 `default`
- **THEN** 校验失败并返回 `E_MANIFEST_SCHEMA`

#### Scenario: 敏感标注被识别

- **WHEN** 某配置项标注 `"sensitive": true`
- **THEN** 校验通过，该标注在解析结果中可被下游读取

### Requirement: 契约描述文件必须由清单类型生成

供编辑器补全使用的 JSON Schema 文件 MUST 从清单的权威类型定义自动生成，MUST NOT 手工维护。

该文件 MUST NOT 被用作校验入口——校验的唯一真值是类型定义本身。

#### Scenario: 生成物与类型保持一致

- **WHEN** 清单类型定义发生变化但生成物未同步
- **THEN** 持续集成失败

#### Scenario: 示例清单可通过校验

- **WHEN** 用设计书中的完整示例清单作为输入
- **THEN** 校验通过，不产生任何错误
