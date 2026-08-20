## Purpose

定义界面描述树被渲染为真实组件时的行为——如何分发、如何在遇到无法识别的内容时降级、以及重绘时如何保持用户正在使用的界面状态不被破坏。

## ADDED Requirements

### Requirement: 渲染器按节点类型递归分发

渲染器 SHALL 依据节点的 `type` 分发到对应组件；容器类组件 SHALL 递归渲染其 `children`。

渲染 MUST 完全由描述树驱动——渲染器 MUST NOT 依赖任何来自插件的可执行代码。

#### Scenario: 嵌套容器被正确展开

- **WHEN** 描述树含三层嵌套的容器组件
- **THEN** 三层结构被完整渲染，层级与顺序与描述树一致

#### Scenario: 非容器组件忽略 children

- **WHEN** 一个非容器类组件的节点携带了 `children`
- **THEN** 该组件正常渲染，`children` 不产生额外输出

### Requirement: 未知组件类型降级为占位符且不影响兄弟节点

节点的 `type` 无法识别时，渲染器 SHALL 在该位置渲染一个错误占位符并记录一条警告。

同一棵树中其余节点 MUST 照常渲染。整棵树 MUST NOT 因单个未知节点而渲染失败。

#### Scenario: 未知节点局部降级

- **WHEN** 一棵含 10 个节点的树中有 1 个未知类型
- **THEN** 9 个节点正常渲染，第 10 个位置显示错误占位符
- **AND** 记录一条警告，指出未识别的类型名

#### Scenario: 占位符可被用户理解

- **WHEN** 错误占位符被渲染
- **THEN** 其内容说明该组件无法识别，而不是留空或显示原始 JSON

### Requirement: 节点标识作为重绘时的比对依据

渲染器 SHALL 以节点的 `id` 作为重绘时的比对依据。

重绘前后 `id` 相同的节点 MUST 保持其焦点状态与滚动位置。

同一棵树中出现重复 `id` 时，渲染器 SHALL 记录一条可定位的警告。

#### Scenario: 焦点在重绘后保持

- **WHEN** 用户正聚焦于某输入框，插件推送一棵新树，该输入框的 `id` 未变
- **THEN** 焦点仍在该输入框上

#### Scenario: 滚动位置在重绘后保持

- **WHEN** 用户已将某列表滚动到中部，插件推送一棵新树，该列表的 `id` 未变
- **THEN** 滚动位置保持不变

#### Scenario: 重复 id 产生可定位的警告

- **WHEN** 一棵树中两个节点使用相同 `id`
- **THEN** 记录警告并指出重复的 `id` 值与出现位置

### Requirement: 交互被打包为约定形状后向上传递

用户交互 SHALL 被打包为含 `nodeId`、`event`、`action`、`value` 四个字段的结构后向上传递。

`action` MUST 取自触发节点 `on` 映射中对应事件名的值；节点未声明该事件时，该交互 MUST NOT 产生上报。

#### Scenario: 未声明的事件不上报

- **WHEN** 用户点击一个 `on` 中没有 `click` 项的按钮
- **THEN** 不产生任何事件上报

#### Scenario: 动作字符串原样透传

- **WHEN** 节点声明 `"on": {"click": "do-sync"}` 且用户点击
- **THEN** 上报的 `action` 恰为 `do-sync`

### Requirement: 富文本与代码组件必须限制可执行内容

Markdown 组件 SHALL 只渲染受限子集（标题、列表、表格、代码块、链接、强调），其中的 HTML 标签 MUST 被转义而非解释。

链接的点击 MUST 由宿主拦截处理，MUST NOT 直接导航。

图片组件 SHALL 接受内联数据形式的图片来源。

#### Scenario: HTML 注入被转义

- **WHEN** Markdown 内容含 `<script>` 或其他 HTML 标签
- **THEN** 这些标签以字面文本显示，不被作为标记解释

#### Scenario: 链接点击被拦截

- **WHEN** 用户点击 Markdown 中的外部链接
- **THEN** 由宿主决定如何处理，页面不直接跳转

#### Scenario: 内联图片可渲染

- **WHEN** 图片组件的来源是内联数据形式
- **THEN** 图片正常显示

### Requirement: 渲染行为可脱离插件独立验证

渲染器的全部行为 SHALL 能够以手写的描述树作为输入进行验证，MUST NOT 依赖插件运行时、沙箱或进程间通信。

#### Scenario: 用样例树驱动验收

- **WHEN** 以覆盖全部组件类型的手写描述树作为输入
- **THEN** 渲染结果可被完整验证，无需加载任何插件
