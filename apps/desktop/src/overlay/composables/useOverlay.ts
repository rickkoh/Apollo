import { computed, onMounted, onScopeDispose, ref } from 'vue'
import * as api from '../../shared/api'
import type { DotState } from '../../shared/components/StatusDot.vue'
import { useHub } from '../../shared/composables/useHub'
import { useListener } from '../../shared/composables/useListener'
import { useSessions } from '../../shared/composables/useSessions'
import { useSettings } from '../../shared/composables/useSettings'
import type { SessionId, UiError } from '../../shared/types'
import { useStack } from './useStack'
import { useTarget } from './useTarget'

const DELIVERED_TICK_MS = 1200

export interface OverlayOptions {
    /** Null keeps messages until newer ones push them out. */
    fadeMs: () => number | null
    /** Puts text that wasn't sent back in the box. */
    restore: (text: string) => void
    onReply?: () => void
    onSent?: () => void
    onError?: () => void
}

/** What every layout shares: the target, its messages, sending and the session picker. */
export function useOverlay(options: OverlayOptions) {
    const pickerOpen = ref(false)
    const delivered = ref(false)
    const dev = ref(false)
    const command = ref('cv')
    api.about().then(info => {
        dev.value = info.channel === 'dev'
        command.value = info.command
    })

    const hub = useHub()
    const { sessions } = useSessions()
    const { target, select } = useTarget(sessions)
    const { settings } = useSettings()
    const { stack, hovered } = useStack(
        target,
        () => ({ fadeMs: options.fadeMs(), maxCards: settings.value.overlay.maxCards }),
        text => {
            options.restore(text)
            options.onError?.()
        },
        () => options.onReply?.(),
    )

    const status = computed<DotState>(() => {
        if (!hub.running.value) return 'hub-down'
        if (stack.pending) return 'pending'
        return target.value?.live ? 'live' : 'offline'
    })
    const otherUnread = computed(() => sessions.value.some(s => s.live && !s.isTarget && s.unread > 0))

    let tickTimer: ReturnType<typeof setTimeout> | undefined
    useListener(api.onMessageUpdated(update => {
        if (update.status !== 'delivered') return
        delivered.value = true
        clearTimeout(tickTimer)
        tickTimer = setTimeout(() => (delivered.value = false), DELIVERED_TICK_MS)
    }))

    function describe(err: unknown): string {
        if (!api.isUiError(err)) return String(err)
        const e: UiError = err
        switch (e.code) {
            case 'no_target':
                return 'Pick a session first'
            case 'target_offline':
                return `${target.value?.title ?? 'That session'} isn't connected`
            case 'too_large':
                return 'That message is too long to send'
            default:
                return e.message
        }
    }

    function showError(err: unknown): void {
        hub.reportError(err)
        stack.error(describe(err))
        options.onError?.()
    }

    async function send(text: string): Promise<void> {
        stack.beginSend()
        try {
            stack.sent(await api.send(text))
            options.onSent?.()
        } catch (err) {
            if (api.isUiError(err) && err.code === 'empty') return
            showError(err)
            options.restore(text)
        }
    }

    async function choose(id: SessionId): Promise<void> {
        pickerOpen.value = false
        try {
            await select(id)
        } catch (err) {
            showError(err)
        }
    }

    function onPointerDown(event: PointerEvent): void {
        if (pickerOpen.value && !(event.target as Element).closest('.picker, .chip')) pickerOpen.value = false
    }
    onMounted(() => document.addEventListener('pointerdown', onPointerDown))
    onScopeDispose(() => document.removeEventListener('pointerdown', onPointerDown))

    function openPanel(): void {
        const id = target.value?.id
        api.openPanel(id ? `/sessions/${encodeURIComponent(id)}` : undefined).catch(() => {})
    }

    return {
        hub,
        sessions,
        target,
        settings,
        stack,
        hovered,
        status,
        otherUnread,
        delivered,
        dev,
        command,
        pickerOpen,
        send,
        choose,
        openPanel,
    }
}
