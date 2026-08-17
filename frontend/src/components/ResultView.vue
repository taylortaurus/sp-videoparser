<script setup>
import { ref, computed } from 'vue'
import Icon from './Icon.vue'
import ImageGallery from './ImageGallery.vue'
import Lightbox from './Lightbox.vue'
import { proxyVideo, proxyImage } from '@/api'
import {
  downloadResource,
  videoFilename,
  coverFilename,
  musicFilename,
} from '@/lib/download'
import { toast } from '@/lib/toast'

const props = defineProps({
  result: { type: Object, required: true },
})

const title = computed(() => props.result.title || '未命名')
const author = computed(() => props.result.author || {})
const images = computed(() => props.result.images || [])
const hasVideo = computed(() => !!props.result.video_url)
const hasImages = computed(() => images.value.length > 0)

const lightboxIndex = ref(null)
const lightboxMode = ref('image')

function openImage(i) {
  lightboxMode.value = 'image'
  lightboxIndex.value = i
}

function openLive(i) {
  lightboxMode.value = 'live'
  lightboxIndex.value = i
}

function dlVideo() {
  downloadResource('video', props.result.video_url, videoFilename(title.value))
  toast('开始下载视频', 'info')
}
function dlCover() {
  downloadResource('image', props.result.cover_url, coverFilename(title.value, props.result.cover_url))
  toast('开始下载封面', 'info')
}
function dlMusic() {
  downloadResource('video', props.result.music_url, musicFilename(title.value, props.result.music_url))
  toast('开始下载音乐', 'info')
}
function onLightboxDownload(i) {
  const img = images.value[i]
  if (img) {
    downloadResource('image', img.url, `${title.value}_${String(i + 1).padStart(3, '0')}.jpg`)
    toast('开始下载图片', 'info')
  }
}

function onLightboxLiveDownload(i) {
  const img = images.value[i]
  if (img?.live_photo_url) {
    downloadResource(
      'video',
      img.live_photo_url,
      `${title.value}_${String(i + 1).padStart(3, '0')}_live.mp4`
    )
    toast('开始下载实况视频', 'info')
  }
}
</script>

<template>
  <section class="result card card-elevated">
    <header class="r-head">
      <h2 class="r-title">{{ title }}</h2>
      <div class="r-author">
        <img
          v-if="author.avatar"
          :src="proxyImage(author.avatar)"
          class="avatar"
          alt=""
          loading="lazy"
        />
        <span v-else class="avatar placeholder"><Icon name="user" :size="16" /></span>
        <div class="a-meta">
          <span class="a-name">{{ author.name || '未知作者' }}</span>
          <span v-if="author.uid" class="a-uid muted mono">{{ author.uid }}</span>
        </div>
      </div>
    </header>

    <div v-if="hasVideo" class="r-block">
      <video class="player" controls preload="metadata" :src="proxyVideo(result.video_url)"></video>
      <div class="block-actions">
        <button class="btn btn-primary btn-sm" @click="dlVideo">
          <Icon name="download" :size="14" /> 下载视频
        </button>
      </div>
    </div>

    <div v-if="result.cover_url || result.music_url" class="r-aux">
      <div v-if="result.cover_url" class="aux-item">
        <img :src="proxyImage(result.cover_url)" class="cover-thumb" alt="封面" loading="lazy" />
        <div class="aux-meta">
          <span class="aux-label">封面</span>
          <button class="btn btn-sm" @click="dlCover">
            <Icon name="download" :size="13" /> 下载
          </button>
        </div>
      </div>
      <div v-if="result.music_url" class="aux-item music">
        <div class="aux-icon"><Icon name="music" :size="18" /></div>
        <div class="aux-meta grow">
          <span class="aux-label">背景音乐</span>
          <audio class="audio" controls preload="none" :src="proxyVideo(result.music_url)"></audio>
        </div>
        <button class="btn btn-sm" @click="dlMusic">
          <Icon name="download" :size="13" /> 下载
        </button>
      </div>
    </div>

    <div v-if="hasImages" class="r-block">
      <ImageGallery
        :images="images"
        :title="title"
        @open="openImage"
        @open-live="openLive"
      />
    </div>

    <Lightbox
      :images="images"
      v-model:index="lightboxIndex"
      v-model:mode="lightboxMode"
      @download="onLightboxDownload"
      @download-live="onLightboxLiveDownload"
    />
  </section>
</template>

<style scoped>
.result {
  padding: var(--sp-6);
  display: flex;
  flex-direction: column;
  gap: var(--sp-5);
}
.r-head {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}
.r-title {
  font-size: var(--fs-lg);
  font-weight: 600;
  line-height: 1.4;
  letter-spacing: -0.01em;
}
.r-author {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}
.avatar {
  width: 36px;
  height: 36px;
  border-radius: 999px;
  object-fit: cover;
  background: var(--bg-subtle);
  border: 1px solid var(--border);
}
.avatar.placeholder {
  display: grid;
  place-items: center;
  color: var(--text-subtle);
}
.a-meta {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}
.a-name {
  font-size: var(--fs-sm);
  font-weight: 500;
}
.a-uid {
  font-size: var(--fs-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 320px;
}

.r-block {
  border-top: 1px solid var(--border);
  padding-top: var(--sp-5);
}
.player {
  width: 100%;
  max-height: 60vh;
  border-radius: var(--radius);
  background: #000;
  display: block;
}
.block-actions {
  margin-top: var(--sp-3);
  display: flex;
  gap: var(--sp-2);
}

.r-aux {
  border-top: 1px solid var(--border);
  padding-top: var(--sp-5);
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--sp-4);
}
.aux-item {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-3);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-subtle);
}
.cover-thumb {
  width: 56px;
  height: 56px;
  object-fit: cover;
  border-radius: var(--radius-sm);
  background: var(--surface);
}
.aux-icon {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border-radius: var(--radius-sm);
  background: var(--surface);
  border: 1px solid var(--border);
  color: var(--text-muted);
}
.aux-meta {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}
.aux-meta.grow {
  flex: 1;
  min-width: 0;
}
.aux-label {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.audio {
  width: 100%;
  height: 32px;
}
@media (max-width: 560px) {
  .r-aux {
    grid-template-columns: 1fr;
  }
}
</style>
