<script setup lang="ts">
import { computed, ref } from 'vue'
import * as api from '../../shared/api'
import { useListener } from '../../shared/composables/useListener'
import { relative } from '../../shared/format'
import type { UpdateStatus } from '../../shared/types'
import SettingRow from './SettingRow.vue'

const status = ref<UpdateStatus | null>(null)
const token = ref('')
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
        error.value = api.isUiError(err) ? err.message : String(err)
    } finally {
        busy.value = false
    }
}

const saveToken = () =>
    run(async () => {
        status.value = await api.saveDeployToken(token.value)
        token.value = ''
    })
const removeToken = () => run(async () => (status.value = await api.saveDeployToken(null)))
const check = () => run(async () => (status.value = await api.checkForUpdates()))
const install = () => run(api.installUpdate)

const state = computed(() => status.value?.state ?? null)
const hint = computed(() => {
    const s = status.value
    if (!s) return 'Checking…'
    if (!s.token) return 'Off: save a GitLab deploy token below to turn updates on'
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
            v-else-if="status?.token"
            class="button"
            type="button"
            :disabled="busy || state?.state === 'checking' || state?.state === 'installing'"
            @click="check"
        >
            Check for updates
        </button>
    </SettingRow>
    <pre v-if="state?.state === 'available' && state.notes" class="notes">{{ state.notes }}</pre>
    <SettingRow
        v-if="status && !status.token"
        label="GitLab deploy token"
        hint="A token with read_package_registry on the Apollo project; kept in the Keychain"
    >
        <input v-model="token" type="password" aria-label="GitLab deploy token" autocomplete="off" @keydown.enter="saveToken" />
        <button class="button" type="button" :disabled="busy || !token.trim()" @click="saveToken">Save</button>
    </SettingRow>
    <SettingRow v-else-if="status" label="GitLab deploy token" hint="Saved in the Keychain">
        <button class="button" type="button" :disabled="busy" @click="removeToken">Remove</button>
    </SettingRow>
    <p v-if="error" class="error-text">{{ error }}</p>
</template>

<style scoped>
input[type='password'] {
    width: 220px;
    padding: 5px 8px;
    border: 1px solid var(--island-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text);
    font: inherit;
}
.notes {
    margin: 0 0 12px;
    padding: 8px 10px;
    border: 1px solid var(--island-border);
    border-radius: 8px;
    background: var(--panel-bg);
    font-family: inherit;
    font-size: 12px;
    white-space: pre-wrap;
}
</style>
