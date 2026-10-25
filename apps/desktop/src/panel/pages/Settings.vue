<script setup lang="ts">
import { computed, ref } from 'vue'
import * as api from '../../shared/api'
import { useCli } from '../../shared/composables/useCli'
import { useHub } from '../../shared/composables/useHub'
import { useSettings } from '../../shared/composables/useSettings'
import type { AboutInfo, ApprovalState, PluginStatus } from '../../shared/types'
import Card from '../components/Card.vue'
import SettingRow from '../components/SettingRow.vue'
import Toggle from '../components/Toggle.vue'

const { settings } = useSettings()
const cli = useCli()
const hub = useHub()
const info = ref<AboutInfo | null>(null)
api.about().then(a => (info.value = a))

const plugin = ref<PluginStatus | null>(null)
const pluginError = ref<string | null>(null)
async function loadPlugin(): Promise<void> {
    pluginError.value = null
    try {
        plugin.value = await api.pluginStatus()
    } catch (err) {
        pluginError.value = api.isUiError(err) ? err.message : String(err)
    }
}
loadPlugin()

async function install(): Promise<void> {
    await cli.install()
    await loadPlugin()
}

const pluginHint = computed(() => {
    if (pluginError.value) return pluginError.value
    if (!plugin.value) return 'Checking…'
    if (!plugin.value.installed) return 'Not installed'
    return plugin.value.upToDate ? 'Installed' : 'Installed, an update is due'
})

const approvalHints: Record<ApprovalState, string> = {
    approved: 'Approved: cv starts without the development-channels dialog',
    missing: "Not approved: cv shows Claude Code's development-channels dialog. Run this once in a terminal",
    wrong_entry: "The approval file doesn't approve the voice plugin. Run this to rewrite it",
    unreadable: "The approval file can't be read. Run this to rewrite it",
    conflict: 'managed-settings.json sets its own approved channels, which can replace this approval. Add the voice plugin there',
}
const approval = computed(() => cli.status.value?.approval ?? null)
const copiedApproval = ref(false)
async function copyApproval(): Promise<void> {
    await navigator.clipboard.writeText(cli.status.value?.approvalCommand ?? '')
    copiedApproval.value = true
    setTimeout(() => (copiedApproval.value = false), 1500)
}

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
                :hint="cli.status.value?.installed ? (cli.status.value.upToDate ? 'Installed' : 'Installed, a reinstall is due') : 'Not installed'"
            >
                <button class="button" type="button" :disabled="cli.busy.value || cli.status.value?.foreign" @click="install">
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
            <SettingRow label="Voice plugin" :hint="pluginHint" />
            <SettingRow v-if="approval" label="Channel approval" :hint="approvalHints[approval]">
                <button v-if="approval !== 'approved' && approval !== 'conflict'" class="button" type="button" @click="copyApproval">
                    {{ copiedApproval ? 'Copied' : 'Copy' }}
                </button>
            </SettingRow>
            <pre v-if="approval && approval !== 'approved' && approval !== 'conflict'" class="command"><code>{{ cli.status.value?.approvalCommand }}</code></pre>
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
.command {
    margin: 0 0 12px;
    padding: 8px 10px;
    border: 1px solid var(--island-border);
    border-radius: 8px;
    background: var(--panel-bg);
    font-size: 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
}
</style>
