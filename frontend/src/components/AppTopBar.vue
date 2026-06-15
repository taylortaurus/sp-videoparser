<script setup>
import Icon from './Icon.vue'
import LogoMark from './LogoMark.vue'
import { useTheme } from '@/lib/theme'
import { LINKS } from '@/lib/links'

defineEmits(['open-platforms'])
const { theme, toggle } = useTheme()
</script>

<template>
  <header class="topbar">
    <div class="container bar-inner">
      <div class="brand">
        <LogoMark :size="24" />
        <span class="brand-name">拾影</span>
        <span class="brand-sub muted">短视频解析</span>
      </div>

      <div class="bar-actions">
        <button class="btn btn-sm btn-ghost" @click="$emit('open-platforms')">
          <Icon name="search" :size="15" />
          <span class="label">支持平台</span>
        </button>
        <span class="bar-sep" aria-hidden="true"></span>
        <a class="btn btn-icon btn-ghost" :href="LINKS.github" target="_blank" rel="noopener" aria-label="GitHub 源码" title="GitHub 源码">
          <Icon name="github" :size="17" />
        </a>
        <a class="btn btn-icon btn-ghost" :href="LINKS.npm" target="_blank" rel="noopener" aria-label="npm 安装" title="npm 安装">
          <Icon name="npm" :size="16" />
        </a>
        <a class="btn btn-icon btn-ghost" :href="LINKS.docker" target="_blank" rel="noopener" aria-label="Docker 镜像" title="Docker 镜像">
          <Icon name="docker" :size="18" />
        </a>
        <span class="bar-sep" aria-hidden="true"></span>
        <button
          class="btn btn-icon btn-ghost"
          :aria-label="theme === 'dark' ? '切换到亮色' : '切换到暗色'"
          :title="theme === 'dark' ? '切换到亮色' : '切换到暗色'"
          @click="toggle"
        >
          <Icon :name="theme === 'dark' ? 'sun' : 'moon'" :size="17" />
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.topbar {
  position: sticky;
  top: 0;
  z-index: var(--z-bar);
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}
.bar-inner {
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.brand {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}
.brand-name {
  font-size: var(--fs-lg);
  font-weight: 650;
  letter-spacing: 0.02em;
}
.brand-sub {
  font-size: var(--fs-xs);
  padding-left: var(--sp-2);
  border-left: 1px solid var(--border);
}
.bar-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}
.bar-sep {
  width: 1px;
  height: 18px;
  background: var(--border);
  margin: 0 4px;
}
@media (max-width: 560px) {
  .brand-sub,
  .bar-actions .label,
  .bar-sep {
    display: none;
  }
}
</style>
