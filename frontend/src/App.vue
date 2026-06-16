<script setup>
import { ref, onMounted, computed, nextTick } from 'vue'
import AppTopBar from './components/AppTopBar.vue'
import ParsePanel from './components/ParsePanel.vue'
import ResultView from './components/ResultView.vue'
import PlatformMarquee from './components/PlatformMarquee.vue'
import FeatureBand from './components/FeatureBand.vue'
import AppFooter from './components/AppFooter.vue'
import ToastHost from './components/ToastHost.vue'
import Icon from './components/Icon.vue'
import { getPlatforms, parseShareUrl, parseById } from './api'
import { toast } from './lib/toast'

const platforms = ref([])
const result = ref(null)
const loading = ref(false)
const error = ref('')

const hasOutcome = computed(() => loading.value || !!error.value || !!result.value)

onMounted(async () => {
  try {
    platforms.value = await getPlatforms()
  } catch {
    // 平台列表加载失败不阻塞解析
  }
})

// 记录最近一次解析任务（thunk），以便失败后「重试」可重新发起
let lastRun = null
const outcomeEl = ref(null)

function scrollToOutcome() {
  const el = outcomeEl.value
  if (!el) return
  // 双帧后瞬时对齐到结果区：瞬时滚动不会被视频/图片异步加载（滚动锚定）打断
  requestAnimationFrame(() =>
    requestAnimationFrame(() => el.scrollIntoView({ block: 'start' }))
  )
}

async function run(task) {
  lastRun = task
  loading.value = true
  error.value = ''
  try {
    result.value = await task()
  } catch (e) {
    result.value = null
    error.value = e.message || '解析失败，请确认链接是否正确'
    toast(error.value, 'error')
  } finally {
    loading.value = false
  }
  // 解析成功后自动滚动到结果展示位置
  if (result.value && !error.value) {
    await nextTick()
    scrollToOutcome()
  }
}
function retry() {
  if (lastRun && !loading.value) run(lastRun)
}
const onParseShare = (text) => run(() => parseShareUrl(text))
const onParseId = (source, videoId) => run(() => parseById(source, videoId))
</script>

<template>
  <div class="aurora" aria-hidden="true"></div>
  <div class="aurora-fade" aria-hidden="true"></div>

  <AppTopBar />

  <main>
    <!-- 平台宣传 ticker：移到标题上方，更好地展示已支持平台 -->
    <PlatformMarquee v-if="platforms.length" :platforms="platforms" class="hero-marquee" />

    <!-- Hero：工具即主角，控制台为焦点 -->
    <section class="hero">
      <div class="container hero-inner">
        <div class="hero-copy">
          <h1 class="hero-title">一个输入框，解决全部素材</h1>
          <div class="steps">
            <span class="step"><i>1</i> 粘贴链接</span>
            <Icon name="arrow" :size="14" class="step-arrow" />
            <span class="step"><i>2</i> 一键解析</span>
            <Icon name="arrow" :size="14" class="step-arrow" />
            <span class="step"><i>3</i> 预览 / 下载</span>
          </div>
        </div>

        <div class="console-wrap">
          <ParsePanel
            :platforms="platforms"
            :loading="loading"
            @parse-share="onParseShare"
            @parse-id="onParseId"
          />

          <ul class="trust">
            <li><Icon name="layers" :size="15" /> 全平台覆盖</li>
            <li><Icon name="play" :size="15" /> 视频 · 图集 · 封面 · 音乐</li>
            <li><Icon name="shield" :size="15" /> 无水印 · 免登录</li>
          </ul>
        </div>

        <div v-if="hasOutcome" ref="outcomeEl" class="outcome">
          <div v-if="loading" class="card card-elevated skeleton" aria-busy="true">
            <div class="sk-line w60"></div>
            <div class="sk-line w30"></div>
            <div class="sk-block"></div>
          </div>

          <div v-else-if="error" class="card card-elevated state error-state" role="alert">
            <Icon name="alert" :size="18" />
            <div class="state-body">
              <p class="state-title">解析失败</p>
              <p class="muted state-msg">{{ error }}</p>
              <div class="state-actions">
                <button class="btn btn-sm" :disabled="loading" @click="retry">重试</button>
              </div>
            </div>
          </div>

          <ResultView v-else-if="result" :result="result" />
        </div>
      </div>
    </section>

    <!-- 能力 / 规格带 -->
    <FeatureBand />
  </main>

  <AppFooter />

  <ToastHost />
</template>

<style scoped>
/* ---- Hero ---- */
.hero {
  padding-top: clamp(var(--sp-4), 3.5vw, var(--sp-6));
  padding-bottom: var(--sp-6);
}
.hero-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
}
.hero-copy {
  max-width: 760px;
  text-align: center;
  margin-bottom: var(--sp-5);
}
.hero-title {
  font-size: var(--fs-hero);
  font-weight: 760;
  letter-spacing: -0.03em;
  line-height: 1.05;
  text-wrap: balance;
}

/* ---- 控制台区 ---- */
.console-wrap {
  width: 100%;
  max-width: 680px;
}
.trust {
  list-style: none;
  margin: var(--sp-3) 0 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--sp-2) var(--sp-4);
}
.trust li {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-muted);
  font-size: var(--fs-sm);
  font-weight: 500;
}
.trust :deep(svg) {
  color: var(--accent);
  opacity: 0.9;
}

.steps {
  margin-top: var(--sp-4);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: var(--sp-2) var(--sp-3);
}
.step {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.step i {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 999px;
  background: var(--accent-soft);
  border: 1px solid var(--accent-border);
  color: var(--accent);
  font-size: 11px;
  font-weight: 700;
  font-style: normal;
}
.step-arrow {
  color: var(--text-subtle);
}

/* ---- 结果 / 状态 ---- */
.outcome {
  width: 100%;
  max-width: 680px;
  margin-top: var(--sp-5);
  scroll-margin-top: 80px;
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
.state-body {
  min-width: 0;
}
.state-actions {
  margin-top: var(--sp-3);
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
  height: 240px;
  border-radius: var(--radius);
  margin-top: var(--sp-2);
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}
@media (prefers-reduced-motion: reduce) {
  .sk-line,
  .sk-block {
    animation: none;
  }
}

/* ---- 平台宣传 ticker（标题上方） ---- */
.hero-marquee {
  margin-bottom: var(--sp-2);
}

/* 入场：自信而克制的分级浮现（reduced-motion 下静止显形） */
@media (prefers-reduced-motion: no-preference) {
  .hero-copy {
    animation: rise 0.7s var(--ease-out) both;
  }
  .console-wrap {
    animation: rise 0.7s var(--ease-out) 0.09s both;
  }
  .outcome {
    animation: rise 0.5s var(--ease-out) both;
  }
}
@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(16px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
</style>
