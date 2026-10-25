import { onScopeDispose, reactive, ref, watch, type Ref } from 'vue'
import * as api from '../../shared/api'
import { useListener } from '../../shared/composables/useListener'
import type { SessionView } from '../../shared/types'
import { MessageStack, type StackOptions } from '../stack'

const TICK_MS = 250

/** The target's message stack: fed by hub events, faded by a timer, paused while hovered. */
export function useStack(
    target: Ref<SessionView | null>,
    options: () => StackOptions,
    onUndelivered: (text: string) => void,
) {
    const stack = reactive(new MessageStack(options()))
    const hovered = ref(false)

    watch(() => target.value?.id, () => stack.clear())
    watch(options, o => stack.configure(o))

    useListener(api.onMessageAdded(message => {
        if (message.session !== target.value?.id) return
        if (message.direction === 'to_session') {
            stack.sent(message)
        } else {
            stack.reply(message)
            api.markRead(message.session, message.id).catch(() => {})
        }
    }))

    useListener(api.onMessageUpdated(({ id, status, detail }) => {
        const failedText = stack.updated(id, status, detail)
        if (failedText !== null) onUndelivered(failedText)
    }))

    const timer = setInterval(() => stack.tick(TICK_MS, hovered.value), TICK_MS)
    onScopeDispose(() => clearInterval(timer))

    return { stack, hovered }
}
