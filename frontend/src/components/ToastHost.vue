<script setup>
import Icon from './Icon.vue'
import { useToasts, dismiss } from '@/lib/toast'

const { items } = useToasts()
const iconFor = { info: 'check', success: 'check', error: 'alert' }
</script>

<template>
  <div class="toast-host" aria-live="polite">
    <TransitionGroup name="toast">
      <div
        v-for="t in items"
        :key="t.id"
        class="toast"
        :class="`toast-${t.type}`"
        @click="dismiss(t.id)"
      >
        <Icon :name="iconFor[t.type] || 'check'" :size="15" />
        <span>{{ t.message }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  left: 50%;
  bottom: var(--sp-5);
  transform: translateX(-50%);
  z-index: var(--z-toast);
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  align-items: center;
  pointer-events: none;
}
.toast {
  pointer-events: auto;
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  max-width: 90vw;
  padding: var(--sp-2) var(--sp-4);
  background: var(--surface);
  color: var(--text);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow-overlay);
  font-size: var(--fs-sm);
  cursor: default;
}
.toast-success {
  border-color: var(--success);
}
.toast-success :deep(svg) {
  color: var(--success);
}
.toast-error {
  border-color: var(--danger);
}
.toast-error :deep(svg) {
  color: var(--danger);
}
.toast-info :deep(svg) {
  color: var(--accent);
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity var(--t), transform var(--t);
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
