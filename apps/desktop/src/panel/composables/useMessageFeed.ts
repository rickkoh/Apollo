import { ref } from 'vue'
import * as api from '../../shared/api'
import { useListener } from '../../shared/composables/useListener'
import type { Message } from '../../shared/types'

/** A list of messages, oldest first, that follows `message-added` (filtered) and `message-updated`. */
export function useMessageFeed(accepts: (message: Message) => boolean, cap = Infinity) {
    const messages = ref<Message[]>([])

    useListener(api.onMessageAdded(message => {
        if (!accepts(message) || messages.value.some(m => m.id === message.id)) return
        messages.value = [...messages.value, message].slice(-cap)
    }))
    useListener(api.onMessageUpdated(({ id, status, detail }) => {
        messages.value = messages.value.map(m => (m.id === id ? { ...m, status, detail } : m))
    }))

    return { messages }
}
