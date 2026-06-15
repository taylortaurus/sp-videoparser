<script setup>
import { ref, computed } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  platforms: { type: Array, default: () => [] },
  open: { type: Boolean, default: false },
})
defineEmits(['close'])

const query = ref('')
const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return props.platforms
  return props.platforms.filter(
    (p) =>
      p.name.toLowerCase().includes(q) ||
      p.source.toLowerCase().includes(q) ||
      (p.domains || []).some((d) => d.toLowerCase().includes(q))
  )
})
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="open" class="modal" @click.self="$emit('close')">
        <div class="dialog card">
          <header class="d-head">
            <h2 class="d-title">支持平台 <span class="muted">{{ platforms.length }}</span></h2>
            <button class="btn btn-icon btn-sm btn-ghost" aria-label="关闭" @click="$emit('close')">
              <Icon name="x" :size="16" />
            </button>
          </header>

          <div class="d-search">
            <Icon name="search" :size="15" />
            <input v-model="query" class="input search-input" placeholder="搜索平台名或域名" spellcheck="false" />
          </div>

          <ul class="plist">
            <li v-for="p in filtered" :key="p.source" class="prow">
              <div class="pname">
                <span class="p-cn">{{ p.name }}</span>
                <span class="p-src mono muted">{{ p.source }}</span>
              </div>
              <span class="p-domains muted">{{ (p.domains || []).join('  ·  ') }}</span>
            </li>
            <li v-if="filtered.length === 0" class="empty muted">无匹配平台</li>
          </ul>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  background: rgba(10, 12, 15, 0.5);
  display: grid;
  place-items: center;
  padding: var(--sp-4);
}
.dialog {
  width: 100%;
  max-width: 560px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-overlay);
}
.d-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-4) var(--sp-5);
  border-bottom: 1px solid var(--border);
}
.d-title {
  font-size: var(--fs-md);
  font-weight: 600;
}
.d-search {
  position: relative;
  padding: var(--sp-3) var(--sp-5);
  border-bottom: 1px solid var(--border);
  color: var(--text-subtle);
}
.d-search :deep(svg) {
  position: absolute;
  left: calc(var(--sp-5) + 10px);
  top: 50%;
  transform: translateY(-50%);
}
.search-input {
  padding-left: 34px;
}
.plist {
  list-style: none;
  margin: 0;
  padding: var(--sp-2) 0;
  overflow-y: auto;
}
.prow {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--sp-4);
  padding: var(--sp-2) var(--sp-5);
}
.prow:hover {
  background: var(--surface-hover);
}
.pname {
  display: flex;
  align-items: baseline;
  gap: var(--sp-2);
  flex-shrink: 0;
}
.p-cn {
  font-size: var(--fs-sm);
  font-weight: 500;
}
.p-src {
  font-size: var(--fs-xs);
}
.p-domains {
  font-size: var(--fs-xs);
  text-align: right;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.empty {
  padding: var(--sp-5);
  text-align: center;
  font-size: var(--fs-sm);
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
