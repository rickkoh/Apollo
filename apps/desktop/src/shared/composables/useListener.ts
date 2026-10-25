import { onScopeDispose } from 'vue'
import type { UnlistenFn } from '../api'

/** Registers an event subscription and removes it when the owning component unmounts. */
export function useListener(subscribe: Promise<UnlistenFn>): void {
    let unlisten: UnlistenFn | null = null
    let disposed = false
    subscribe.then(fn => (disposed ? fn() : (unlisten = fn)))
    onScopeDispose(() => {
        disposed = true
        unlisten?.()
    })
}
