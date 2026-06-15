// 轻量滚动渐显指令：进入视口时淡入上移。尊重 prefers-reduced-motion。
export const reveal = {
  mounted(el, binding) {
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    el.classList.add('reveal')
    if (binding.value && binding.value.delay) {
      el.style.transitionDelay = `${binding.value.delay}ms`
    }
    if (reduce) {
      el.classList.add('in')
      return
    }
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            el.classList.add('in')
            io.unobserve(el)
          }
        })
      },
      { threshold: 0.12, rootMargin: '0px 0px -6% 0px' }
    )
    io.observe(el)
    el._revealIO = io
  },
  unmounted(el) {
    if (el._revealIO) el._revealIO.disconnect()
  },
}
