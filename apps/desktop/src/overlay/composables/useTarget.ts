import { computed, type Ref } from 'vue'
import * as api from '../../shared/api'
import type { SessionId, SessionView } from '../../shared/types'

/** The target follows `sessions-changed`, which the hub emits with every target change. */
export function useTarget(sessions: Ref<SessionView[]>) {
    const target = computed(() => sessions.value.find(s => s.isTarget) ?? null)
    const select = (id: SessionId) => api.setTarget(id)
    return { target, select }
}
