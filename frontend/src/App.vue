<script setup>
import { ref, onMounted, computed } from 'vue'
import AppTopBar from './components/AppTopBar.vue'
import ParsePanel from './components/ParsePanel.vue'
import ResultView from './components/ResultView.vue'
import PlatformMarquee from './components/PlatformMarquee.vue'
import AppFooter from './components/AppFooter.vue'
import PlatformsDialog from './components/PlatformsDialog.vue'
import ToastHost from './components/ToastHost.vue'
import Icon from './components/Icon.vue'
import { getPlatforms, parseShareUrl, parseById } from './api'
import { toast } from './lib/toast'

const platforms = ref([])
const result = ref(null)
const loading = ref(false)
const error = ref('')
const showPlatforms = ref(false)

const hasOutcome = computed(() => loading.value || !!error.value || !!result.value)

onMounted(async () => {
  try {
    platforms.value = await getPlatforms()
  } catch {
    // 平台列表加载失败不阻塞解析
  }
})

async function run(promise) {
  loading.value = true
  error.value = ''
  try {
    result.value = await promise
  } catch (e) {
    result.value = null
    error.value = e.message || '解析失败'
    toast(error.value, 'error')
  } finally {
    loading.value = false
  }
}
const onParseShare = (text) => run(parseShareUrl(text))
const onParseId = (source, videoId) => run(parseById(source, videoId))
</script>

<template>
  <AppTopBar @open-platforms="showPlatforms = true" />

  <main>
    <!-- Hero：工具即主角，可直接操作 -->
    <section class="hero">
      <div class="container hero-inner">
        <div class="hero-copy">
          <h1 class="hero-title">全平台短视频，去水印解析</h1>
          <div class="usage">
            <span class="u-step"><span class="u-n">1</span>粘贴链接</span>
            <Icon name="arrow" :size="15" class="u-arrow" />
            <span class="u-step"><span class="u-n">2</span>一键解析</span>
            <Icon name="arrow" :size="15" class="u-arrow" />
            <span class="u-step"><span class="u-n">3</span>预览 / 下载</span>
          </div>
        </div>

        <ParsePanel
          :platforms="platforms"
          :loading="loading"
          @parse-share="onParseShare"
          @parse-id="onParseId"
        />

        <ul class="stats">
          <li><strong>26</strong> 个平台</li>
          <li><strong>视频 · 图集 · 音乐</strong></li>
          <li><strong>无水印 · 免登录</strong></li>
        </ul>

        <div v-if="hasOutcome" class="outcome">
          <div v-if="loading" class="card skeleton" aria-busy="true">
            <div class="sk-line w60"></div>
            <div class="sk-line w30"></div>
            <div class="sk-block"></div>
          </div>

          <div v-else-if="error" class="card state error-state">
            <Icon name="alert" :size="18" />
            <div>
              <p class="state-title">解析失败</p>
              <p class="muted state-msg">{{ error }}</p>
            </div>
          </div>

          <ResultView v-else-if="result" :result="result" />
        </div>
      </div>
    </section>

    <PlatformMarquee :platforms="platforms" />
  </main>

  <AppFooter />

  <PlatformsDialog :platforms="platforms" :open="showPlatforms" @close="showPlatforms = false" />
  <ToastHost />
</template>

<style scoped>
.hero {
  padding-top: var(--sp-6);
  padding-bottom: var(--sp-5);
}
.hero-inner {
  display: flex;
  flex-direction: column;
}
.hero-copy {
  max-width: 56ch;
  margin: 0 auto var(--sp-5);
  text-align: center;
}
.hero-title {
  font-size: clamp(26px, 4.4vw, 36px);
  font-weight: 700;
  letter-spacing: -0.025em;
  line-height: 1.2;
}
.usage {
  margin-top: var(--sp-4);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: var(--sp-2) var(--sp-3);
}
.u-step {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: var(--fs-sm);
  font-weight: 500;
  color: var(--text-muted);
}
.u-n {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 999px;
  background: var(--accent-soft);
  border: 1px solid var(--accent-border);
  color: var(--accent);
  font-size: 11px;
  font-weight: 600;
}
.u-arrow {
  color: var(--text-subtle);
}

.stats {
  list-style: none;
  margin: var(--sp-4) 0 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--sp-2) var(--sp-5);
  color: var(--text-muted);
  font-size: var(--fs-sm);
}
.stats li {
  position: relative;
}
.stats li + li::before {
  content: '';
  position: absolute;
  left: calc(var(--sp-5) / -2 - 1px);
  top: 50%;
  transform: translateY(-50%);
  width: 1px;
  height: 12px;
  background: var(--border-strong);
}
.stats strong {
  color: var(--text);
  font-weight: 600;
}

.outcome {
  margin-top: var(--sp-6);
}
.state {
  padding: var(--sp-5);
}
.state-title {
  font-size: var(--fs-md);
  font-weight: 600;
}
.state-msg {
  margin-top: var(--sp-2);
  font-size: var(--fs-sm);
}
.error-state {
  display: flex;
  gap: var(--sp-3);
  align-items: flex-start;
  border-color: var(--danger);
  background: var(--danger-soft);
}
.error-state :deep(svg) {
  color: var(--danger);
  flex-shrink: 0;
  margin-top: 2px;
}

/* 骨架屏 */
.skeleton {
  padding: var(--sp-5);
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}
.sk-line,
.sk-block {
  background: var(--bg-subtle);
  border-radius: var(--radius-sm);
  animation: pulse 1.4s ease-in-out infinite;
}
.sk-line {
  height: 14px;
}
.sk-line.w60 {
  width: 60%;
}
.sk-line.w30 {
  width: 30%;
}
.sk-block {
  height: 220px;
  border-radius: var(--radius);
  margin-top: var(--sp-2);
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.55;
  }
}
@media (prefers-reduced-motion: reduce) {
  .sk-line,
  .sk-block {
    animation: none;
  }
}
</style>
