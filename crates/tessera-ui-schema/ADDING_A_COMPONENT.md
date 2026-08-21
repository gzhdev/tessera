# 新增一个组件

本 crate 是 UI 描述树契约的**权威来源**（设计书 §17.1 约束 5）。新增组件的
正确顺序是固定的，且顺序错了会被工具挡住：

```
改类型 → 重新生成 → 实现渲染
```

## 步骤

1. **改类型**（本 crate）
   - 在 `src/components.rs` 新增 `XxxProps` struct：`#[derive(...)]` +
     `#[serde(rename_all = "camelCase", deny_unknown_fields)]` +
     `#[ts(export_to = "XxxProps.ts", rename_all = "camelCase")]`。
   - 在 `src/node.rs` 的 `Component` 枚举加变体（`#[serde(rename = "...")]` +
     `#[ts(rename = "...")]`，标记用 kebab-case 逐变体显式指定），并在
     `type_tag()` 补一行、`is_container()`（若为容器）补一行。
   - 在 `tests/fixtures/positive/xxx.json` 加正例 fixture，并在
     `tests/components.rs` 用 `positive_case!` 注册一条。

2. **重新生成**（契约同步到前端）

   ```bash
   cargo run -p tessera-ui-schema --bin gen-types
   ```

   产出 `src/types/generated/XxxProps.ts` 并更新 `UiNode.ts` 的判别联合。
   生成物 check-in 到 git；CI 校验「重新生成后与已提交内容逐字节一致」。

3. **实现渲染**（`ui-component-library`）
   - 在 `src/components/ui/` 加一个 `.vue`，在 `<UiNode>` 递归渲染器的
     分发里登记 `case "xxx"`。

## 顺序错了会被什么挡住

| 遗漏 | 拦截点 |
|---|---|
| 改了类型没重新生成 | CI「UI 类型生成物一致性」步：`git diff --exit-code src/types/generated/` 非零 |
| 手改了生成物 | 同上：重新生成会覆盖手改，diff 检出漂移 |
| 前端先实现 `.vue` 再补类型 | TS 类型检查：渲染分发引用不存在的判别标记 / props 类型编译失败 |
| props 字段拼错 / 多了契约外字段 | `deny_unknown_fields`：校验期拒绝，测试与宿主校验均报「属性类型不符」 |

组件清单一旦发布即受 `CORE_API_LEVEL` 约束（§13.2）：**加**组件是兼容变更，
**删**组件或改 props 语义要 +1。因此新增时宁可保守（见 design.md Risks）。
