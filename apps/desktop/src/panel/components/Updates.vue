<script setup lang="ts">
import { computed, ref } from 'vue'
import * as api from '../../shared/api'
import { useListener } from '../../shared/composables/useListener'
import { relative } from '../../shared/format'
import type { UpdateStatus } from '../../shared/types'
import SettingRow from './SettingRow.vue'

const status = ref<UpdateStatus | null>(null)
const busy = ref(false)
const error = ref<string | null>(null)

api.updateStatus().then(s => (status.value = s))
useListener(api.onUpdateStatus(s => (status.value = s)))

async function run(action: () => Promise<unknown>): Promise<void> {
    busy.value = true
    error.value = null
    try {
        await action()
    } catch (err) {
        error.value = api.errorMessage(err)
    } finally {
        busy.value = false
    }
}

const check = () => run(async () => (status.value = await api.checkForUpdates()))
const install = () => run(api.installUpdate)

const state = computed(() => status.value?.state ?? null)
const hint = computed(() => {
    const s = status.value
    if (!s) return 'Checking…'
    switch (s.state.state) {
        case 'idle':
            return 'Not checked yet'
        case 'checking':
            return 'Checking…'
        case 'up_to_date':
            return `Up to date, checked ${relative(s.checkedAt)}`
        case 'available':
            return `Version ${s.state.version} is available`
        case 'installing':
            return `Installing ${s.state.version}; Apollo relaunches when it's done`
        case 'failed':
            return `Couldn't update: ${s.state.message}`
    }
})
</script>

<template>
    <SettingRow label="Updates" :hint="hint">
        <button
            v-if="state?.state === 'available'"
            class="button"
            type="button"
            :disabled="busy"
            @click="install"
        >
            Update to v{{ state.version }}
        </button>
        <button
            v-else-if="status"
            class="button"
            type="button"
            :disabled="busy || state?.state === 'checking' || state?.state === 'installing'"
            @click="check"
        >
            Check for updates
        </button>
    </SettingRow>
    <pre v-if="state?.state === 'available' && state.notes" class="notes">{{ state.notes }}</pre>
    <p v-if="error" class="error-text">{{ error }}</p>
</template>

<style scoped>
.notes {
    margin: 0 0 12px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--control);
    font-family: inherit;
    font-size: 12px;
    white-space: pre-wrap;
}
</style>
