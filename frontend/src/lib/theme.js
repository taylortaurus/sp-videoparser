import { ref } from 'vue'

// 主题：亮色优先；无用户偏好时跟随系统。手动切换持久化到 localStorage。
const KEY = 'vp-theme'
const theme = ref('light')

function apply(t) {
  document.documentElement.setAttribute('data-theme', t)
}

export function initTheme() {
  const saved = localStorage.getItem(KEY)
  if (saved === 'light' || saved === 'dark') {
    theme.value = saved
  } else {
    theme.value = window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light'
  }
  apply(theme.value)
}

export function useTheme() {
  function toggle() {
    theme.value = theme.value === 'dark' ? 'light' : 'dark'
    localStorage.setItem(KEY, theme.value)
    apply(theme.value)
  }
  return { theme, toggle }
}
