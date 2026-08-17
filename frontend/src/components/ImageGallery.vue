<script setup>
import { ref, computed, watch } from 'vue'
import Icon from './Icon.vue'
import { proxyImage } from '@/api'
import { downloadImagesZip, downloadImagesEach } from '@/lib/download'
import { toast } from '@/lib/toast'

const props = defineProps({
  images: { type: Array, default: () => [] },
  title: { type: String, default: 'images' },
})
const emit = defineEmits(['open', 'open-live'])

const picked = ref(props.images.map(() => false))
watch(
  () => props.images,
  (imgs) => {
    picked.value = imgs.map(() => false)
  }
)

const selectedCount = computed(() => picked.value.filter(Boolean).length)
const allSelected = computed(
  () => props.images.length > 0 && selectedCount.value === props.images.length
)

function toggleAll() {
  const v = !allSelected.value
  picked.value = props.images.map(() => v)
}

// 有勾选则只下勾选项，否则全部
function targetImages() {
  if (selectedCount.value === 0) return props.images
  return props.images.filter((_, i) => picked.value[i])
}

const zipping = ref(false)
async function onZip() {
  if (zipping.value) return
  const list = targetImages()
  zipping.value = true
  toast(`正在打包 ${list.length} 张图片`, 'info', 1800)
  try {
    await downloadImagesZip(list, props.title)
    toast('打包完成，已开始下载', 'success')
  } catch (e) {
    toast(`打包失败：${e.message}`, 'error')
  } finally {
    zipping.value = false
  }
}

async function onEach() {
  const list = targetImages()
  toast(`开始逐张下载 ${list.length} 张`, 'info', 1800)
  try {
    await downloadImagesEach(list, props.title)
  } catch (e) {
    toast(`下载失败：${e.message}`, 'error')
  }
}

</script>

<template>
  <div class="gallery">
    <div class="gallery-bar">
      <div class="gallery-meta">
        <span class="gallery-count">图集 {{ images.length }} 张</span>
        <span v-if="selectedCount" class="muted">已选 {{ selectedCount }}</span>
      </div>
      <div class="gallery-tools">
        <button class="btn btn-sm btn-ghost" @click="toggleAll">
          {{ allSelected ? '取消全选' : '全选' }}
        </button>
        <button class="btn btn-sm" :disabled="zipping" @click="onZip">
          <Icon v-if="zipping" name="loader" :size="14" class="spin" />
          <Icon v-else name="archive" :size="14" />
          打包下载
        </button>
        <button class="btn btn-sm btn-ghost" @click="onEach">
          <Icon name="download" :size="14" />
          逐张
        </button>
      </div>
    </div>

    <div class="grid">
      <figure v-for="(img, i) in images" :key="i" class="cell" :class="{ on: picked[i] }">
        <button class="thumb" type="button" @click="emit('open', i)">
          <img :src="proxyImage(img.url)" loading="lazy" :alt="`图片 ${i + 1}`" />
        </button>
        <label class="pick" @click.stop>
          <input type="checkbox" v-model="picked[i]" />
          <span class="box"><Icon name="check" :size="12" /></span>
        </label>
        <button
          v-if="img.live_photo_url"
          class="live"
          type="button"
          title="预览实况照片"
          :aria-label="`预览第 ${i + 1} 张实况照片`"
          @click.stop="emit('open-live', i)"
        >
          <Icon name="play" :size="12" /> 预览实况
        </button>
      </figure>
    </div>
  </div>
</template>

<style scoped>
.gallery-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-3);
  margin-bottom: var(--sp-3);
}
.gallery-meta {
  display: flex;
  align-items: baseline;
  gap: var(--sp-3);
}
.gallery-count {
  font-size: var(--fs-sm);
  font-weight: 500;
}
.gallery-meta .muted {
  font-size: var(--fs-xs);
}
.gallery-tools {
  display: flex;
  gap: var(--sp-2);
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(116px, 1fr));
  gap: var(--sp-2);
}
.cell {
  position: relative;
  margin: 0;
  border-radius: var(--radius);
  overflow: hidden;
  border: 1px solid var(--border);
  background: var(--bg-subtle);
}
.cell.on {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-soft);
}
.thumb {
  display: block;
  width: 100%;
  aspect-ratio: 1;
  padding: 0;
  border: 0;
  background: var(--bg-subtle);
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.pick {
  position: absolute;
  top: 6px;
  left: 6px;
  cursor: pointer;
}
.pick input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}
.pick .box {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 5px;
  background: var(--surface);
  border: 1px solid var(--border-strong);
  color: transparent;
}
.pick input:checked + .box {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--accent-contrast);
}
.live {
  position: absolute;
  bottom: 6px;
  right: 6px;
  display: inline-flex;
  align-items: center;
  gap: 3px;
  min-height: 26px;
  padding: 3px 7px;
  border: 0;
  border-radius: 5px;
  background: rgba(0, 0, 0, 0.6);
  color: #fff;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0;
  transition: background var(--t);
}
.live:hover {
  background: rgba(0, 0, 0, 0.78);
}
@media (max-width: 560px) {
  .gallery-bar {
    align-items: stretch;
    flex-direction: column;
  }
  .gallery-tools {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .gallery-tools .btn {
    min-width: 0;
    padding-inline: var(--sp-2);
  }
  .grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
