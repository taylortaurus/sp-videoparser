<script setup>
// 品牌标识：圆角方块内「播放三角 + 底部托盘」，寓意"预览 + 保存"。
// 白色字形配品牌蓝底，亮暗主题皆清晰；亦用作 favicon。
defineProps({
  size: { type: [Number, String], default: 24 },
})
</script>

<template>
  <svg :width="size" :height="size" viewBox="0 0 32 32" aria-hidden="true">
    <rect width="32" height="32" rx="8.5" fill="var(--accent)" />
    <path d="M12.4 9.2 L22.2 16 L12.4 22.8 Z" fill="#fff" />
    <rect x="9.4" y="24.6" width="13.2" height="2.4" rx="1.2" fill="#fff" fill-opacity="0.55" />
  </svg>
</template>
