import { computed, ref } from 'vue'
import * as api from '../api'
import type { SessionView } from '../types'
import { useListener } from './useListener'

export function useSessions() {
    const sessions = ref<SessionView[]>([])
    const live = computed(() => sessions.value.filter(s => s.live))
    useListener(api.onSessionsChanged(next => (sessions.value = next)))
    api.listSessions().then(list => (sessions.value = list), () => {})
    return { sessions, live }
}
