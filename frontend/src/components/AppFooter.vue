<script setup>
// 页脚仅保留使用说明与许可信息；项目链接已上移至顶栏。
</script>

<template>
  <footer class="footer">
    <div class="container foot-inner">
      <p class="foot-note muted">
        本工具仅供学习与技术研究，请勿用于商业或非法用途，并遵守各平台服务条款；内容版权归原作者所有。
      </p>
      <p class="foot-meta muted">拾影 · MIT License</p>
    </div>
  </footer>
</template>

<style scoped>
.footer {
  border-top: 1px solid var(--border);
  background: var(--bg);
}
.foot-inner {
  padding-top: var(--sp-5);
  padding-bottom: var(--sp-5);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-3);
}
.foot-note {
  font-size: var(--fs-sm);
  line-height: 1.6;
  max-width: 70ch;
}
.foot-meta {
  font-size: var(--fs-xs);
  white-space: nowrap;
}
@media (max-width: 640px) {
  .foot-inner {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-2);
  }
}
</style>
