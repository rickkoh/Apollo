import { onMounted, onScopeDispose, type Ref } from 'vue'
import * as api from '../../shared/api'

/** Keeps the overlay window as tall as its islands (D19), at most once per animation frame. */
export function useAutoResize(root: Ref<HTMLElement | null>): void {
    let frame = 0
    let last = 0
    const observer = new ResizeObserver(() => {
        cancelAnimationFrame(frame)
        frame = requestAnimationFrame(() => {
            const height = Math.ceil(root.value?.getBoundingClientRect().height ?? 0)
            if (height > 0 && height !== last) {
                last = height
                api.overlayResize(height).catch(() => {})
            }
        })
    })
    onMounted(() => root.value && observer.observe(root.value))
    onScopeDispose(() => {
        cancelAnimationFrame(frame)
        observer.disconnect()
    })
}
