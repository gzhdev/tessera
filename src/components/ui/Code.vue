<script setup lang="ts">
// code：语法高亮（highlight.js core + 按需注册语言，任务 6.4）。
// 按需加载：只用到的语言才 import 并 registerLanguage——全语言包 5.5 MB，
// core 76 KB + 单语言数 KB。未注册的语言回退为转义纯文本（spec 未要求高亮
// 全部语言，只要求「语言按需加载」）。highlight.js 输出本身已转义，可安全 v-html。
import { computed } from 'vue'
import hljs from 'highlight.js/lib/core'
import rust from 'highlight.js/lib/languages/rust'
import typescript from 'highlight.js/lib/languages/typescript'
import javascript from 'highlight.js/lib/languages/javascript'
import json from 'highlight.js/lib/languages/json'
import toml from 'highlight.js/lib/languages/ini' // toml 语法与 ini 同族，复用 ini
import bash from 'highlight.js/lib/languages/bash'
import yaml from 'highlight.js/lib/languages/yaml'
import type { NodeProps } from './nodeProps'

// 注册本组件库实际用到的语言（fixture 覆盖范围内）；新增语言在此追加一行即可。
hljs.registerLanguage('rust', rust)
hljs.registerLanguage('typescript', typescript)
hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('json', json)
hljs.registerLanguage('toml', toml)
hljs.registerLanguage('bash', bash)
hljs.registerLanguage('yaml', yaml)

const props = withDefaults(defineProps<NodeProps<'code'>>(), { path: '/root' })

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

const rendered = computed(() => {
  const lang = props.node.props.language
  if (lang !== undefined && hljs.getLanguage(lang) !== undefined) {
    return hljs.highlight(props.node.props.code, { language: lang }).value
  }
  // 未声明或未注册的语言：纯文本（转义），不高亮也不出错。
  return escapeHtml(props.node.props.code)
})
</script>

<template>
  <pre class="ui-code" :data-language="node.props.language"><code class="hljs" v-html="rendered"></code></pre>
</template>
