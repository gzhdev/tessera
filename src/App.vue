<script setup lang="ts">
// 占位页：脚手架验证用（core-workspace-bootstrap 任务 6.2）。
// 真实的停靠面板/插件管理界面由后续 ui-component-library 等 change 实现。
</script>

<template>
  <main class="placeholder">
    <h1>Tessera</h1>
    <p>桌面插件化工具核心 · 工程基座已就位（前端为占位页）</p>
  </main>
</template>

<style>
body {
  margin: 0;
  font-family: 'Segoe UI', 'Microsoft YaHei', system-ui, sans-serif;
  background: #1e1f24;
  color: #e6e6e6;
}

.placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  gap: 0.5rem;
}

h1 {
  margin: 0;
  letter-spacing: 0.3em;
}

p {
  opacity: 0.65;
}
</style>
