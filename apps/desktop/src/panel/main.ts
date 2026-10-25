import { createApp } from 'vue'
import '../shared/tokens.css'
import { provideSettings } from '../shared/composables/useSettings'
import App from './App.vue'
import { router } from './router'

const app = createApp(App).use(router)
await provideSettings(app)
app.mount('#app')
