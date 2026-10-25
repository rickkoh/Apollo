import { ref } from 'vue'
import * as api from '../api'
import type { CliStatus } from '../types'

export function useCli() {
    const status = ref<CliStatus | null>(null)
    const error = ref<string | null>(null)
    const busy = ref(false)

    async function refresh(): Promise<void> {
        status.value = await api.cliStatus()
    }

    async function install(): Promise<void> {
        busy.value = true
        error.value = null
        try {
            status.value = await api.installCli()
        } catch (err) {
            error.value = api.isUiError(err) ? err.message : String(err)
        } finally {
            busy.value = false
        }
    }

    refresh().catch(err => (error.value = api.isUiError(err) ? err.message : String(err)))
    return { status, error, busy, install, refresh }
}
