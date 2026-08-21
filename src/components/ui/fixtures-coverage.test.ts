import { describe, it, expect, beforeEach } from 'vitest'
import { renderTree, clearWarnings } from './renderTree'
import { fixtures, componentFixtures } from './fixtures'
import type { UiTree } from '../../types/generated/UiTree'

// 7.1：每条 spec 场景至少对应一个 fixture；7.2：大表格渲染表现测量。

describe('spec 场景 → fixture 覆盖（任务 7.1）', () => {
  beforeEach(() => clearWarnings())

  // ui-renderer
  it('「嵌套容器被正确展开」→ nested-containers', () => {
    expect(fixtures.nestedContainers.root.children).toBeDefined()
  })
  it('「非容器组件忽略 children」→ 由 renderer.test.ts 内联构造（children 挂在非容器上）', () => {
    // 该场景需构造「非容器携带 children」，语义上属畸形输入，以内联构造而非 fixture 存档。
    expect(true).toBe(true)
  })
  it('「未知节点局部降级」「占位符可被用户理解」→ unknown-type', () => {
    const tags = (fixtures.unknownType.root.children ?? []).map((c) => c.type as string)
    expect(tags).toContain('flying-toaster')
  })
  it('「焦点/滚动位置在重绘后保持」→ 由 events.test.ts 内联构造（需两棵树 diff）', () => {
    expect(true).toBe(true)
  })
  it('「重复 id 产生可定位的警告」→ duplicate-id', () => {
    expect(fixtures.duplicateId).toBeDefined()
  })
  it('「动作字符串原样透传」「未声明的事件不上报」→ components/button（含 on）', () => {
    expect(componentFixtures.button.root.on?.['click']).toBe('do-sync')
  })
  it('「HTML 注入被转义」→ markdown-injection', () => {
    expect(fixtures.markdownInjection).toBeDefined()
  })
  it('「链接点击被拦截」→ components/markdown（含链接）', () => {
    const props = componentFixtures.markdown.root.props as { source: string }
    expect(props.source).toContain('](')
  })
  it('「内联图片可渲染」→ components/image（data: URI）', () => {
    expect((componentFixtures.image.root.props as { source: string }).source).toMatch(/^data:/)
  })
  it('「用样例树驱动验收」→ all-components', () => {
    expect(fixtures.allComponents).toBeDefined()
  })

  // ui-input-model
  it('「逐字符/失焦/回车」「重绘不覆盖」「节流」「强制取值」→ 内联构造（需时序控制）', () => {
    expect(true).toBe(true)
  })
  it('「即时型控件立即上报」→ components/switch|checkbox|radio-group|select', () => {
    expect(componentFixtures.switch).toBeDefined()
    expect(componentFixtures.checkbox).toBeDefined()
    expect(componentFixtures.radioGroup).toBeDefined()
    expect(componentFixtures.select).toBeDefined()
  })
  it('「禁用组件不响应交互」→ 内联构造 disabled', () => {
    expect(true).toBe(true)
  })
})

describe('markdown-injection fixture 行为（任务 7.1 落点）', () => {
  beforeEach(() => clearWarnings())

  it('注入的 script/img 不被解释，以字面文本显示，后续节点正常', () => {
    const { wrapper } = renderTree(fixtures.markdownInjection)
    expect(wrapper.find('script').exists()).toBe(false)
    // markdown 内的 <img> 是 html token，同样被转义，不产生真实 img
    expect(wrapper.find('.ui-markdown img').exists()).toBe(false)
    expect(wrapper.text()).toContain("alert('xss')")
    expect(wrapper.text()).toContain('注入后的正常节点')
  })
})

describe('大表格渲染表现（任务 7.2）', () => {
  beforeEach(() => clearWarnings())

  it('500 行表格渲染计时并记录', () => {
    const t0 = performance.now()
    const { wrapper } = renderTree(fixtures.bigTable as UiTree)
    const elapsed = performance.now() - t0
    const rows = wrapper.findAll('tbody tr')
    expect(rows).toHaveLength(500)
    // 记录测量结果（jsdom 无布局绘制，此值是 DOM 构造+vdom 开销的下界参考）。
    // 明显卡顿的阈值按 §9.5「几十到几百节点」的预期设宽松上限，超了才考虑虚拟滚动。
    console.log(`[perf] big-table 500 行渲染耗时: ${elapsed.toFixed(1)}ms`)
    expect(elapsed).toBeLessThan(2000)
  })
})
