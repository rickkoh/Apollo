import { createApp } from 'vue'
import '../shared/tokens.css'
import { provideSettings } from '../shared/composables/useSettings'
import Overlay from './Overlay.vue'

const app = createApp(Overlay)
await provideSettings(app)
app.mount('#app')
