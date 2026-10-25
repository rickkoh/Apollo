import { onScopeDispose, reactive, watch, watchEffect } from 'vue'
import * as api from '../../shared/api'
import { useListener } from '../../shared/composables/useListener'
import { Island } from '../island'

export interface IslandOptions {
    news: () => boolean
    peekMs: () => number
    /** Runs once the panel holds the keyboard, to focus the text box. */
    onOpen: () => void
}

/** Runs `Island` on real timers and moves the keyboard with it: taken on open, given back before closing animates. */
export function useIsland(options: IslandOptions) {
    const island = reactive(new Island())
    let timer: ReturnType<typeof setTimeout> | undefined

    function schedule(): void {
        clearTimeout(timer)
        const due = island.due
        if (due === null) return
        timer = setTimeout(() => {
            island.advance(performance.now())
            schedule()
        }, Math.max(0, due - performance.now()))
    }

    function run(change: (now: number) => void): void {
        change(performance.now())
        schedule()
    }

    watchEffect(() => (island.news = options.news()))
    useListener(api.onOverlayPointer(({ inside }) => run(now => island.pointer(inside, now))))
    useListener(api.onOverlaySummon(({ released }) => run(now => (released ? island.released(now) : island.summon()))))

    const onFocus = () => run(now => island.keyboard(true, now))
    const onBlur = () => run(now => island.keyboard(false, now))
    window.addEventListener('focus', onFocus)
    window.addEventListener('blur', onBlur)
    onScopeDispose(() => {
        clearTimeout(timer)
        window.removeEventListener('focus', onFocus)
        window.removeEventListener('blur', onBlur)
    })

    watch(
        () => island.state,
        (to, from) => {
            if (to === 'open') api.overlayKeyboard(true).then(options.onOpen, () => {})
            else if (from === 'open') api.overlayKeyboard(false).catch(() => {})
        },
    )

    return {
        island,
        escape: () => run(() => island.escape()),
        sent: () => run(() => island.sent()),
        reply: () => run(now => island.reply(now, options.peekMs())),
        error: () => run(now => island.error(now, options.peekMs())),
    }
}
