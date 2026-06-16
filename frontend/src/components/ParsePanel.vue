<script setup>
import { ref, computed } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  platforms: { type: Array, default: () => [] },
  loading: { type: Boolean, default: false },
})
const emit = defineEmits(['parse-share', 'parse-id'])

const text = ref('')
const advanced = ref(false)
const source = ref('')
const videoId = ref('')

// 从文案中提取首个 URL（与后端正则一致的简化版）
const extractedUrl = computed(() => {
  const m = text.value.match(/https?:\/\/[^\s]+/)
  return m ? m[0] : ''
})

// 依据 URL 域名匹配平台
const detected = computed(() => {
  const url = extractedUrl.value
  if (!url) return null
  return (
    props.platforms.find((p) => (p.domains || []).some((d) => url.includes(d))) ||
    null
  )
})

function submitShare() {
  if (!text.value.trim() || props.loading) return
  emit('parse-share', text.value.trim())
}

function submitId() {
  if (!source.value || !videoId.value.trim() || props.loading) return
  emit('parse-id', source.value, videoId.value.trim())
}

function onKeydown(e) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    submitShare()
  }
}
</script>

<template>
  <section class="card card-elevated parse">
    <!-- 命令栏式输入：前置链接图标 + 多行输入 -->
    <div class="field">
      <Icon name="link" :size="18" class="field-icon" />
      <textarea
        id="share-input"
        v-model="text"
        class="field-input"
        rows="2"
        aria-label="粘贴分享链接或整段文案"
        placeholder="粘贴抖音 / 快手 / B站… 的分享链接或整段文案，回车解析"
        spellcheck="false"
        @keydown="onKeydown"
      ></textarea>
    </div>

    <div class="parse-actions">
      <div class="detect">
        <span v-if="detected" class="detect-pill ok">
          <span class="dot"><Icon name="check" :size="11" /></span>
          已识别 <b>{{ detected.name }}</b>
        </span>
        <span v-else-if="extractedUrl" class="detect-pill warn">
          <span class="dot">?</span>
          未识别平台 · 仍可解析
        </span>
        <span v-else class="detect-pill idle">
          <span class="dot"><Icon name="link" :size="11" /></span>
          粘贴后自动识别平台
        </span>
        <button class="adv-toggle" type="button" @click="advanced = !advanced">
          <Icon name="sliders" :size="14" />
          按 ID 解析
        </button>
      </div>

      <button
        class="btn btn-primary btn-lg parse-go"
        :disabled="!text.trim() || loading"
        @click="submitShare"
      >
        <template v-if="loading">
          <Icon name="loader" :size="16" class="spin" /> 解析中
        </template>
        <template v-else>
          解析 <Icon name="arrow" :size="16" class="go-arrow" />
        </template>
      </button>
    </div>

    <div v-if="advanced" class="advanced">
      <div class="adv-row">
        <div class="adv-field">
          <label class="adv-label" for="src">平台</label>
          <select id="src" v-model="source" class="select">
            <option value="" disabled>选择平台</option>
            <option v-for="p in platforms" :key="p.source" :value="p.source">
              {{ p.name }} ({{ p.source }})
            </option>
          </select>
        </div>
        <div class="adv-field grow">
          <label class="adv-label" for="vid">视频 ID</label>
          <input id="vid" v-model="videoId" class="input" placeholder="平台内的视频 ID" spellcheck="false" />
        </div>
        <button
          class="btn"
          :disabled="!source || !videoId.trim() || loading"
          @click="submitId"
        >
          解析 ID
        </button>
      </div>
      <p class="muted adv-note">部分平台不支持按 ID 解析，此时请使用上方的链接解析。</p>
    </div>
  </section>
</template>

<style scoped>
.parse {
  padding: var(--sp-5);
  transition: box-shadow var(--t-med);
}
.parse:focus-within {
  box-shadow: var(--shadow-lg);
}

/* ---- 命令栏式输入 ---- */
.field {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-3);
  padding: var(--sp-3) var(--sp-4);
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  transition: border-color var(--t), box-shadow var(--t);
}
.field:focus-within {
  border-color: var(--accent);
  box-shadow: var(--ring-focus);
}
.field-icon {
  color: var(--text-subtle);
  margin-top: 4px;
  flex-shrink: 0;
  transition: color var(--t);
}
.field:focus-within .field-icon {
  color: var(--accent);
}
.field-input {
  flex: 1;
  min-width: 0;
  border: 0;
  background: transparent;
  outline: none;
  resize: vertical;
  min-height: 48px;
  font-family: inherit;
  font-size: var(--fs-md);
  line-height: 1.55;
  color: var(--text);
}
.field-input::placeholder {
  color: var(--text-subtle);
}

/* ---- 动作行 ---- */
.parse-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-3);
  margin-top: var(--sp-3);
}
.detect {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-wrap: wrap;
  min-width: 0;
}

/* 平台识别提示：三态精致胶囊 */
.detect-pill {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 28px;
  padding: 0 12px 0 5px;
  border-radius: 999px;
  border: 1px solid transparent;
  font-size: var(--fs-xs);
  font-weight: 500;
  white-space: nowrap;
}
.detect-pill .dot {
  display: grid;
  place-items: center;
  width: 19px;
  height: 19px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 700;
}
.detect-pill.ok {
  background: var(--accent-soft);
  border-color: var(--accent-border);
  color: var(--accent);
}
.detect-pill.ok .dot {
  background: var(--accent);
  color: var(--accent-contrast);
}
.detect-pill.ok b {
  font-weight: 700;
}
.detect-pill.warn {
  background: var(--surface);
  border-color: var(--border-strong);
  color: var(--text-muted);
}
.detect-pill.warn .dot {
  background: var(--bg-subtle);
  color: var(--text-muted);
}
.detect-pill.idle {
  background: transparent;
  border-color: var(--border);
  color: var(--text-subtle);
}
.detect-pill.idle .dot {
  background: var(--bg-subtle);
  color: var(--text-subtle);
}

.adv-toggle {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 0;
  background: transparent;
  color: var(--text-muted);
  font-size: var(--fs-xs);
  font-weight: 500;
  padding: 4px 2px;
  border-radius: var(--radius-sm);
}
.adv-toggle:hover {
  color: var(--accent);
}

/* 解析按钮：图标 + 悬停箭头微动 */
.parse-go {
  flex-shrink: 0;
}
.go-arrow {
  transition: transform var(--t);
}
.parse-go:hover:not(:disabled) .go-arrow {
  transform: translateX(3px);
}

/* ---- 按 ID 解析 ---- */
.advanced {
  margin-top: var(--sp-4);
  padding-top: var(--sp-4);
  border-top: 1px solid var(--border);
}
.adv-row {
  display: flex;
  align-items: flex-end;
  gap: var(--sp-3);
}
.adv-label {
  display: block;
  font-size: var(--fs-sm);
  font-weight: 500;
  color: var(--text-muted);
  margin-bottom: var(--sp-2);
}
.adv-field {
  min-width: 0;
}
.adv-field.grow {
  flex: 1;
}
.adv-field .select {
  min-width: 160px;
}
.adv-note {
  font-size: var(--fs-xs);
  margin-top: var(--sp-2);
}
@media (max-width: 560px) {
  .parse-actions {
    flex-direction: column;
    align-items: stretch;
  }
  .parse-go {
    width: 100%;
  }
  .adv-row {
    flex-wrap: wrap;
  }
  .adv-field.grow {
    flex: 1 1 100%;
  }
}
</style>
