<script setup lang="ts">
// markdown：受限子集（标题、列表、表格、代码块、链接、强调，设计书 §9.6）。
// HTML 标签一律转义为字面文本（spec「HTML 注入被转义」）——renderer 层把 html
// token 转义，插件侧没有任何开关可打开 HTML 解释（D6：默认行为，非配置项）。
// 链接点击由宿主拦截（spec「链接点击被拦截」）：统一在容器上捕获 click。
import { computed } from 'vue'
import { Marked } from 'marked'
import { useLinkClickHandler } from './linkClick'
import type { NodeProps } from './nodeProps'

const props = withDefaults(defineProps<NodeProps<'markdown'>>(), { path: '/root' })
const onLinkClick = useLinkClickHandler()

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

// 单例解析器：配置只在模块加载时建一次。
const marked = new Marked({
  gfm: true, // 表格来自 GFM
  breaks: false,
  renderer: {
    // HTML 不解释，转义为字面文本（含 <script>）。
    html({ raw }) { return escapeHtml(raw) },
    // 图片不在受限子集内（图片由 image 组件承担），降级为字面文本。
    image({ href, title, text }) {
      return escapeHtml(`![${text}](${href}${title ? ` "${title}"` : ''})`)
    },
    // 链接加 data-md-link 标记，点击在容器层拦截交给宿主。
    link({ href, tokens }) {
      const text = this.parser.parseInline(tokens)
      return `<a href="${escapeHtml(href)}" data-md-link="1">${text}</a>`
    },
  },
})

const html = computed(() => marked.parse(props.node.props.source, { async: false }))

function onClick(e: MouseEvent): void {
  const anchor = (e.target as HTMLElement).closest('a[data-md-link]')
  if (!anchor) return
  // 宿主决定如何处理；页面绝不直接跳转（spec「链接点击被拦截」）。
  e.preventDefault()
  const href = anchor.getAttribute('href')
  if (href) onLinkClick(href)
}
</script>

<template>
  <!-- html 由受限解析器产出，其中 HTML 已转义为字面文本 -->
  <div class="ui-markdown" @click="onClick" v-html="html"></div>
</template>
