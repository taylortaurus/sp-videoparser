<script setup>
import Icon from './Icon.vue'

const features = [
  {
    icon: 'shield',
    term: '无水印 · 高清直链',
    desc: '直取平台源地址，去除水印，保留原清晰度。',
  },
  {
    icon: 'layers',
    term: '视频 · 图集 · 封面 · 音乐',
    desc: '一次解析，拿到一条内容的全部可下载素材。',
  },
  {
    icon: 'zap',
    term: '免登录 · 免安装',
    desc: '粘贴即用，浏览器里直接在线预览与下载。',
  },
  {
    icon: 'server',
    term: '全平台一站式',
    desc: '抖音 / 快手 / B 站 / 小红书 / 微博…持续更新。',
  },
]
</script>

<template>
  <section class="section feature">
    <div class="container">
      <div class="grid card card-elevated">
        <div v-for="f in features" :key="f.term" class="cell">
          <span class="f-icon"><Icon :name="f.icon" :size="20" /></span>
          <h3 class="f-term">{{ f.term }}</h3>
          <p class="f-desc muted">{{ f.desc }}</p>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.feature {
  padding-top: var(--sp-7);
}
.grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 1px;
  background: var(--border);
  overflow: hidden;
}
.cell {
  background: var(--surface);
  padding: var(--sp-5);
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}
.f-icon {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border-radius: var(--radius);
  background: var(--accent-soft);
  border: 1px solid var(--accent-border);
  color: var(--accent);
  margin-bottom: var(--sp-2);
}
.f-term {
  font-size: var(--fs-md);
  font-weight: 650;
  letter-spacing: -0.01em;
  line-height: 1.3;
}
.f-desc {
  font-size: var(--fs-sm);
  line-height: 1.6;
}

@media (max-width: 860px) {
  .grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
@media (max-width: 480px) {
  .grid {
    grid-template-columns: 1fr;
  }
}
</style>
