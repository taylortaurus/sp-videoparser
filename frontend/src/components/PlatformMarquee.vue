<script setup>
import { computed } from 'vue'

const props = defineProps({
  platforms: { type: Array, default: () => [] },
})

// 复制一份以实现无缝循环（轨道平移 -50% 即回到起点）
const doubled = computed(() =>
  props.platforms.length ? [...props.platforms, ...props.platforms] : []
)
</script>

<template>
  <div v-if="platforms.length" class="marquee" aria-hidden="true">
    <div class="mq-track">
      <span v-for="(p, i) in doubled" :key="i" class="mq-item">{{ p.name }}</span>
    </div>
  </div>
</template>

<style scoped>
.marquee {
  overflow: hidden;
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  padding: var(--sp-3) 0;
  -webkit-mask-image: linear-gradient(
    90deg,
    transparent,
    #000 7%,
    #000 93%,
    transparent
  );
  mask-image: linear-gradient(90deg, transparent, #000 7%, #000 93%, transparent);
}
.mq-track {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  width: max-content;
  animation: mq 52s linear infinite;
}
.marquee:hover .mq-track {
  animation-play-state: paused;
}
.mq-item {
  display: inline-flex;
  align-items: center;
  height: 30px;
  padding: 0 var(--sp-3);
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg);
  font-size: var(--fs-sm);
  color: var(--text-muted);
  white-space: nowrap;
}
@keyframes mq {
  to {
    transform: translateX(-50%);
  }
}
@media (prefers-reduced-motion: reduce) {
  .marquee {
    overflow-x: auto;
  }
  .mq-track {
    animation: none;
  }
}
</style>
