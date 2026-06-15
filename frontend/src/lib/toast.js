import { ref } from 'vue'

// 轻量 toast：用于瞬时反馈（复制成功、下载开始、错误提示）。
let seq = 0
const items = ref([])

export function useToasts() {
  return { items }
}

export function toast(message, type = 'info', timeout = 2800) {
  const id = ++seq
  items.value.push({ id, message, type })
  if (timeout > 0) {
    setTimeout(() => dismiss(id), timeout)
  }
  return id
}

export function dismiss(id) {
  items.value = items.value.filter((t) => t.id !== id)
}
