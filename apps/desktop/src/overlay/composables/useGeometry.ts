import { ref } from 'vue'
import * as api from '../../shared/api'
import { useListener } from '../../shared/composables/useListener'
import type { OverlayGeometry } from '../../shared/types'

/** Where the shell hangs the overlay and the notch to draw around, following screen and layout changes. */
export function useGeometry() {
    const geometry = ref<OverlayGeometry | null>(null)
    api.overlayGeometry().then(g => (geometry.value ??= g), () => {})
    useListener(api.onOverlayGeometry(g => (geometry.value = g)))
    return geometry
}
