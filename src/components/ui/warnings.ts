import type { Warning } from '../../types/generated/Warning'

// 渲染期警告收集器（design.md D2：映射表未命中即记警告；§9.3：重复 id 可定位警告）。
// 模块级单例，渲染时由各组件 push；测试用 renderTree.ts 的 clearWarnings 复位。
// 宿主侧（core-ui-bridge）接入时可替换为正式的日志通道。
export const uiWarnings: Warning[] = []

export function reportWarning(warning: Warning): void {
  uiWarnings.push(warning)
}
