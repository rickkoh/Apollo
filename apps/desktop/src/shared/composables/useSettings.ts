import { ref } from 'vue'
import * as api from '../api'
import type { Settings } from '../types'
import { useListener } from './useListener'

export function useSettings() {
    const settings = ref<Settings | null>(null)
    api.getSettings().then(s => (settings.value = s))
    useListener(api.onSettingsChanged(s => (settings.value = s)))
    return { settings }
}
