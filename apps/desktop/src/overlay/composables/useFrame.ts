import { onMounted, onScopeDispose, ref, watch, type Ref } from 'vue'
import * as api from '../../shared/api'

export interface Size {
    width: number
    height: number
}

/** Keeps the overlay window at `size`, at most once per animation frame. */
export function useFrame(size: () => Size): void {
    let frame = 0
    let last = ''
    watch(
        size,
        next => {
            cancelAnimationFrame(frame)
            frame = requestAnimationFrame(() => {
                const width = Math.ceil(next.width)
                const height = Math.ceil(next.height)
                const key = `${width}x${height}`
                if (width <= 0 || height <= 0 || key === last) return
                last = key
                api.overlayFrame(width, height).catch(() => {})
            })
        },
        { immediate: true },
    )
    onScopeDispose(() => cancelAnimationFrame(frame))
}

export function useElementSize(el: Ref<HTMLElement | null>): Ref<Size> {
    const size = ref<Size>({ width: 0, height: 0 })
    const observer = new ResizeObserver(() => {
        const rect = el.value?.getBoundingClientRect()
        if (rect) size.value = { width: rect.width, height: rect.height }
    })
    onMounted(() => el.value && observer.observe(el.value))
    onScopeDispose(() => observer.disconnect())
    return size
}
