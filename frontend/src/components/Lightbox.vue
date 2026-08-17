<script setup>
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import Icon from './Icon.vue'
import { proxyImage, proxyVideo } from '@/api'

const props = defineProps({
  images: { type: Array, default: () => [] },
  index: { type: Number, default: null },
  mode: { type: String, default: 'image' },
})
const emit = defineEmits(['update:index', 'update:mode', 'download', 'download-live'])

const open = computed(() => props.index !== null && props.index >= 0)
const current = computed(() => (open.value ? props.images[props.index] : null))
const canPlayLive = computed(() => !!current.value?.live_photo_url)
const showingLive = computed(() => props.mode === 'live' && canPlayLive.value)
const videoFailed = ref(false)

watch(
  () => [props.index, props.mode],
  () => {
    videoFailed.value = false
    if (props.mode === 'live' && !canPlayLive.value) emit('update:mode', 'image')
  }
)

function close() {
  emit('update:index', null)
}
function setMode(mode) {
  if (mode === 'live' && !canPlayLive.value) return
  videoFailed.value = false
  emit('update:mode', mode)
}
function prev() {
  if (props.index > 0) emit('update:index', props.index - 1)
}
function next() {
  if (props.index < props.images.length - 1) emit('update:index', props.index + 1)
}
function onKey(e) {
  if (!open.value) return
  if (e.key === 'Escape') close()
  else if (e.key === 'ArrowLeft') prev()
  else if (e.key === 'ArrowRight') next()
  else if (e.key.toLowerCase() === 'l' && canPlayLive.value) {
    setMode(showingLive.value ? 'image' : 'live')
  }
}
onMounted(() => window.addEventListener('keydown', onKey))
onUnmounted(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="open" class="lb" @click.self="close">
        <div class="lb-bar">
          <span class="lb-count mono">{{ index + 1 }} / {{ images.length }}</span>
          <div v-if="canPlayLive" class="mode-switch" role="group" aria-label="预览模式">
            <button
              type="button"
              :class="{ active: !showingLive }"
              :aria-pressed="!showingLive"
              @click="setMode('image')"
            >
              <Icon name="image" :size="14" /> 照片
            </button>
            <button
              type="button"
              :class="{ active: showingLive }"
              :aria-pressed="showingLive"
              @click="setMode('live')"
            >
              <Icon name="play" :size="13" /> 实况
            </button>
          </div>
          <div class="lb-actions">
            <button
              class="btn btn-sm"
              @click="showingLive ? emit('download-live', index) : emit('download', index)"
            >
              <Icon name="download" :size="14" />
              {{ showingLive ? '下载实况' : '下载此图' }}
            </button>
            <button class="btn btn-icon btn-sm" aria-label="关闭" @click="close">
              <Icon name="x" :size="16" />
            </button>
          </div>
        </div>

        <button
          v-if="index > 0"
          class="nav nav-prev"
          aria-label="上一张"
          @click="prev"
        >
          <Icon name="chevron" :size="22" style="transform: rotate(90deg)" />
        </button>

        <div v-if="current" class="media-stage">
          <video
            v-if="showingLive && !videoFailed"
            :key="current.live_photo_url"
            class="lb-media"
            :src="proxyVideo(current.live_photo_url)"
            :poster="proxyImage(current.url)"
            controls
            autoplay
            muted
            loop
            playsinline
            preload="metadata"
            @error="videoFailed = true"
          ></video>
          <div v-else-if="showingLive && videoFailed" class="media-error" role="status">
            <img :src="proxyImage(current.url)" class="lb-media" :alt="`图片 ${index + 1}`" />
            <p>实况预览加载失败，可直接下载后查看</p>
          </div>
          <img
            v-else
            :src="proxyImage(current.url)"
            class="lb-media"
            :alt="`图片 ${index + 1}`"
          />
        </div>

        <button
          v-if="index < images.length - 1"
          class="nav nav-next"
          aria-label="下一张"
          @click="next"
        >
          <Icon name="chevron" :size="22" style="transform: rotate(-90deg)" />
        </button>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.lb {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  background: rgba(10, 12, 15, 0.86);
  display: grid;
  place-items: center;
  padding: calc(64px + env(safe-area-inset-top))
    max(var(--sp-6), env(safe-area-inset-right))
    max(var(--sp-4), env(safe-area-inset-bottom))
    max(var(--sp-6), env(safe-area-inset-left));
}
.lb-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  min-height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: env(safe-area-inset-top) max(var(--sp-4), env(safe-area-inset-right)) 0
    max(var(--sp-4), env(safe-area-inset-left));
}
.lb-count {
  color: rgba(255, 255, 255, 0.8);
  font-size: var(--fs-sm);
}
.lb-actions {
  display: flex;
  gap: var(--sp-2);
}
.mode-switch {
  position: absolute;
  left: 50%;
  top: calc(env(safe-area-inset-top) + 10px);
  transform: translateX(-50%);
  display: inline-flex;
  padding: 3px;
  border-radius: var(--radius);
  background: rgba(255, 255, 255, 0.12);
}
.mode-switch button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-width: 76px;
  height: 30px;
  padding: 0 var(--sp-3);
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: rgba(255, 255, 255, 0.76);
  font-size: var(--fs-xs);
  font-weight: 600;
  transition: background var(--t), color var(--t);
}
.mode-switch button.active {
  background: #fff;
  color: #161a20;
}
.media-stage {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: grid;
  place-items: center;
}
.lb-media {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: var(--radius);
}
.media-error {
  position: relative;
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
}
.media-error p {
  position: absolute;
  bottom: var(--sp-4);
  left: 50%;
  transform: translateX(-50%);
  width: max-content;
  max-width: calc(100% - 32px);
  padding: 7px 10px;
  border-radius: var(--radius);
  background: rgba(10, 12, 15, 0.78);
  color: #fff;
  font-size: var(--fs-xs);
  text-align: center;
}
.nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
  transition: background var(--t);
}
.nav:hover {
  background: rgba(255, 255, 255, 0.22);
}
.nav-prev {
  left: var(--sp-4);
}
.nav-next {
  right: var(--sp-4);
}
.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--t);
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
@media (max-width: 640px) {
  .lb {
    padding-top: calc(108px + env(safe-area-inset-top));
    padding-inline: max(var(--sp-3), env(safe-area-inset-left));
  }
  .lb-bar {
    min-height: 100px;
    align-items: flex-start;
    padding-top: calc(env(safe-area-inset-top) + var(--sp-3));
  }
  .lb-count {
    line-height: 32px;
  }
  .mode-switch {
    top: calc(env(safe-area-inset-top) + 56px);
    width: calc(100% - 24px);
  }
  .mode-switch button {
    flex: 1;
  }
  .lb-actions .btn:not(.btn-icon) {
    width: 44px;
    padding: 0;
    font-size: 0;
  }
  .nav {
    width: 36px;
    height: 44px;
    border-radius: var(--radius);
  }
  .nav-prev {
    left: var(--sp-2);
  }
  .nav-next {
    right: var(--sp-2);
  }
}
</style>
