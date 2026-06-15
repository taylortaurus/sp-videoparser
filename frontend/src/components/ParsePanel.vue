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
  <section class="card parse">
    <label class="field-label" for="share-input">粘贴分享链接或整段文案</label>
    <textarea
      id="share-input"
      v-model="text"
      class="textarea"
      rows="3"
      placeholder="例如：将抖音/快手/B站等的分享文案直接粘贴到这里，回车解析"
      spellcheck="false"
      @keydown="onKeydown"
    ></textarea>

    <div class="parse-actions">
      <div class="detect">
        <span v-if="detected" class="tag">
          <Icon name="check" :size="13" />
          已识别 · {{ detected.name }}
        </span>
        <span v-else-if="extractedUrl" class="muted detect-hint">
          未识别平台，仍可尝试解析
        </span>
        <button class="adv-toggle muted" type="button" @click="advanced = !advanced">
          <Icon name="sliders" :size="14" />
          按 ID 解析
        </button>
      </div>
      <button class="btn btn-primary" :disabled="!text.trim() || loading" @click="submitShare">
        <Icon v-if="loading" name="loader" :size="15" class="spin" />
        {{ loading ? '解析中' : '解析' }}
      </button>
    </div>

    <div v-if="advanced" class="advanced">
      <div class="adv-row">
        <div class="adv-field">
          <label class="field-label" for="src">平台</label>
          <select id="src" v-model="source" class="select">
            <option value="" disabled>选择平台</option>
            <option v-for="p in platforms" :key="p.source" :value="p.source">
              {{ p.name }} ({{ p.source }})
            </option>
          </select>
        </div>
        <div class="adv-field grow">
          <label class="field-label" for="vid">视频 ID</label>
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
}
.field-label {
  display: block;
  font-size: var(--fs-sm);
  font-weight: 500;
  color: var(--text-muted);
  margin-bottom: var(--sp-2);
}
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
  min-width: 0;
}
.detect-hint {
  font-size: var(--fs-xs);
}
.adv-toggle {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 0;
  background: transparent;
  font-size: var(--fs-xs);
  padding: 0;
}
.adv-toggle:hover {
  color: var(--text);
}
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
  .adv-row {
    flex-wrap: wrap;
  }
  .adv-field.grow {
    flex: 1 1 100%;
  }
}
</style>
