<script setup lang="ts">
import { ref } from 'vue'
import * as api from '../../shared/api'
import { useCli } from '../../shared/composables/useCli'
import { useHub } from '../../shared/composables/useHub'
import { useSettings } from '../../shared/composables/useSettings'
import type { AboutInfo } from '../../shared/types'
import Card from '../components/Card.vue'
import SettingRow from '../components/SettingRow.vue'
import Toggle from '../components/Toggle.vue'

const { settings } = useSettings()
const cli = useCli()
const hub = useHub()
const info = ref<AboutInfo | null>(null)
api.about().then(a => (info.value = a))

const error = ref<string | null>(null)
const confirmingClear = ref(false)
const cleared = ref(false)

async function patch(change: object): Promise<void> {
    error.value = null
    try {
        settings.value = await api.updateSettings(change)
    } catch (err) {
        error.value = api.isUiError(err) ? err.message : String(err)
        settings.value = await api.getSettings()
    }
}

function numberFrom(event: Event): number {
    return Number((event.target as HTMLInputElement).value)
}

function reveal(): void {
    api.revealDataFolder().catch(err => (error.value = api.isUiError(err) ? err.message : String(err)))
}

async function clearAll(): Promise<void> {
    confirmingClear.value = false
    await api.clearHistory()
    cleared.value = true
}
</script>

<template>
    <h1>Settings</h1>
    <p v-if="error" class="error-text" role="alert">{{ error }}</p>
    <template v-if="settings">
        <h2>General</h2>
        <Card>
            <SettingRow label="Show the overlay at launch">
                <Toggle
                    :model-value="settings.overlay.visibleAtLaunch"
                    label="Show the overlay at launch"
                    @update:model-value="v => patch({ overlay: { visibleAtLaunch: v } })"
                />
            </SettingRow>
        </Card>

        <h2>Overlay</h2>
        <Card>
            <SettingRow label="Reply preview lines" hint="Lines of a reply shown before “more” (1–8)">
                <input
                    type="number"
                    min="1"
                    max="8"
                    aria-label="Reply preview lines"
                    :value="settings.overlay.replyPreviewLines"
                    @change="e => patch({ overlay: { replyPreviewLines: numberFrom(e) } })"
                />
            </SettingRow>
        </Card>

        <h2>Terminal command</h2>
        <Card>
            <SettingRow
                label="cv"
                :hint="cli.status.value?.installed ? (cli.status.value.upToDate ? 'Installed' : 'Installed, points at another copy of the app') : 'Not installed'"
            >
                <button class="button" type="button" :disabled="cli.busy.value || cli.status.value?.foreign" @click="cli.install">
                    {{ cli.status.value?.installed ? 'Reinstall' : 'Install' }}
                </button>
            </SettingRow>
            <p class="dim path"><code>{{ cli.status.value?.path }}</code></p>
            <p v-if="cli.status.value?.foreign" class="error-text">
                A <code>cv</code> that Voice Bridge didn't write is already there; remove or rename it to install.
            </p>
            <p v-if="cli.status.value && !cli.status.value.onPath" class="warn">
                <code>~/.local/bin</code> isn't on your <code>PATH</code>. Add
                <code>export PATH="$HOME/.local/bin:$PATH"</code> to your shell profile.
            </p>
            <p v-if="cli.error.value" class="error-text">{{ cli.error.value }}</p>
        </Card>

        <h2>History</h2>
        <Card>
            <SettingRow label="Keep history for" hint="Older messages are removed at startup (1–365 days)">
                <input
                    type="number"
                    min="1"
                    max="365"
                    aria-label="Days to keep history"
                    :value="settings.history.retentionDays"
                    @change="e => patch({ history: { retentionDays: numberFrom(e) } })"
                />
                <span class="dim">days</span>
            </SettingRow>
            <SettingRow label="Clear all history" :hint="cleared ? 'History cleared' : 'Removes every message; sessions stay'">
                <button v-if="confirmingClear" class="button danger" type="button" @click="clearAll">Clear everything?</button>
                <button v-else class="button" type="button" @click="confirmingClear = true">Clear…</button>
            </SettingRow>
        </Card>

        <h2>About</h2>
        <Card>
            <SettingRow label="Version"><span class="dim">{{ info?.version }}</span></SettingRow>
            <SettingRow label="Hub port">
                <span class="dim">{{ hub.running.value ? (hub.port.value ?? '…') : 'not running' }}</span>
            </SettingRow>
            <SettingRow label="Data folder" :hint="info?.dataDir">
                <button class="button" type="button" @click="reveal">Reveal in Finder</button>
            </SettingRow>
        </Card>
    </template>
</template>

<style scoped>
input[type='number'] {
    width: 64px;
    padding: 5px 8px;
    border: 1px solid var(--island-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text);
    font: inherit;
}
.path {
    margin: 4px 0 0;
    font-size: 12px;
}
.warn {
    color: var(--warn);
}
</style>
