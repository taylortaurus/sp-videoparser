import { createApp } from 'vue'
import App from './App.vue'
import './styles/tokens.css'
import './styles/base.css'
import { initTheme } from './lib/theme'
import { reveal } from './lib/reveal'

initTheme()
const app = createApp(App)
app.directive('reveal', reveal)
app.mount('#app')
