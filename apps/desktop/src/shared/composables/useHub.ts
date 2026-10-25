import { ref } from 'vue'
import * as api from '../api'
import { useListener } from './useListener'

/** Whether the in-app hub is running; false once it stops or a command reports it down. */
export function useHub() {
    const running = ref(true)
    const port = ref<number | null>(null)
    api.hubStatus().then(
        status => (port.value = status.port),
        err => (running.value = !(api.isUiError(err) && err.code === 'hub_down')),
    )
    useListener(api.onHubState(state => (running.value = state.running)))
    return { running, port }
}
