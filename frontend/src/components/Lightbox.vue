<script setup>
import { computed, onMounted, onUnmounted } from 'vue'
import Icon from './Icon.vue'
import { proxyImage } from '@/api'

const props = defineProps({
  images: { type: Array, default: () => [] },
  index: { type: Number, default: null },
})
const emit = defineEmits(['update:index', 'download'])

const open = computed(() => props.index !== null && props.index >= 0)
const current = computed(() => (open.value ? props.images[props.index] : null))

function close() {
  emit('update:index', null)
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
          <div class="lb-actions">
            <button class="btn btn-sm" @click="emit('download', index)">
              <Icon name="download" :size="14" /> 下载此图
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

        <img v-if="current" :src="proxyImage(current.url)" class="lb-img" :alt="`图片 ${index + 1}`" />

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
  padding: 64px var(--sp-6);
}
.lb-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--sp-4);
}
.lb-count {
  color: rgba(255, 255, 255, 0.8);
  font-size: var(--fs-sm);
}
.lb-actions {
  display: flex;
  gap: var(--sp-2);
}
.lb-img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: var(--radius);
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
</style>
