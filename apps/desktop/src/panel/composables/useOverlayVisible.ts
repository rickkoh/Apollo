import { ref } from 'vue'
import * as api from '../../shared/api'
import { useListener } from '../../shared/composables/useListener'

export function useOverlayVisible() {
    const visible = ref(false)
    api.overlayVisible().then(v => (visible.value = v))
    useListener(api.onOverlayVisibility(({ visible: v }) => (visible.value = v)))
    const set = (next: boolean) => api.setOverlayVisible(next)
    return { visible, set }
}
