<script setup lang="ts">
import { computed, ref } from 'vue'
import * as api from '../../shared/api'
import { useCli } from '../../shared/composables/useCli'
import { useHub } from '../../shared/composables/useHub'
import { useSettings } from '../../shared/composables/useSettings'
import type { AboutInfo, ApprovalState, EditorPanel, IdePanelStatus, PluginStatus } from '../../shared/types'
import Card from '../components/Card.vue'
import SettingRow from '../components/SettingRow.vue'
import Toggle from '../components/Toggle.vue'
import Updates from '../components/Updates.vue'
import { useCopy } from '../composables/useCopy'

const props = defineProps<{ section: string }>()
const { settings } = useSettings()
const cli = useCli()
const hub = useHub()
const info = ref<AboutInfo | null>(null)
const stable = computed(() => info.value?.channel === 'stable')

const plugin = ref<PluginStatus | null>(null)
const pluginError = ref<string | null>(null)
async function loadPlugin(): Promise<void> {
    pluginError.value = null
    try {
        plugin.value = await api.pluginStatus()
    } catch (err) {
        pluginError.value = api.errorMessage(err)
    }
}

const idePanel = ref<IdePanelStatus | null>(null)
const loadIdePanel = async () => (idePanel.value = await api.idePanelStatus())

api.about().then(a => {
    info.value = a
    if (stable.value) Promise.all([loadPlugin(), loadIdePanel()])
})

async function install(): Promise<void> {
    await cli.install()
    if (stable.value) await Promise.all([loadPlugin(), loadIdePanel()])
}

const idePanelHint = computed(() => {
    if (!idePanel.value) return 'Checking…'
    if (idePanel.value.foreign) return "A cv-panel that Apollo didn't write is in the way"
    if (!idePanel.value.installed) return 'Not installed: click Install above'
    return idePanel.value.upToDate ? 'Installed' : 'Installed, a reinstall is due'
})

function editorHint(editor: EditorPanel): string {
    switch (editor.wrapper) {
        case 'ours':
            return 'Set up: panel sessions started from now on get voice'
        case 'unset':
            return 'Add the line below to its settings.json (Preferences: Open User Settings (JSON))'
        case 'other':
            return `Uses another wrapper (${editor.current}); to use Apollo's, replace it with the line below`
    }
}

const settingCopy = useCopy()
const copySetting = () => settingCopy.copy(idePanel.value?.setting ?? '')

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
const approvalCopy = useCopy()
const copyApproval = () => approvalCopy.copy(cli.status.value?.approvalCommand ?? '')

const error = ref<string | null>(null)
const confirmingClear = ref(false)

const openAtLogin = ref<boolean | null>(null)
api.openAtLogin().then(v => (openAtLogin.value = v))
async function setOpenAtLogin(enabled: boolean): Promise<void> {
    error.value = null
    try {
        openAtLogin.value = await api.setOpenAtLogin(enabled)
    } catch (err) {
        error.value = api.errorMessage(err)
    }
}
const cleared = ref(false)

async function patch(change: object): Promise<void> {
    error.value = null
    try {
        settings.value = await api.updateSettings(change)
    } catch (err) {
        error.value = api.errorMessage(err)
        settings.value = await api.getSettings()
    }
}

function numberFrom(event: Event): number {
    return Number((event.target as HTMLInputElement).value)
}

function reveal(): void {
    api.revealDataFolder().catch(err => (error.value = api.errorMessage(err)))
}

async function clearAll(): Promise<void> {
    confirmingClear.value = false
    await api.clearHistory()
    cleared.value = true
}
</script>

<template>
    <p v-if="error" class="error-text" role="alert">{{ error }}</p>
    <template v-if="settings && props.section === 'general'">
        <h1>General</h1>
        <Card>
            <SettingRow label="Show the overlay at launch">
                <Toggle
                    :model-value="settings.overlay.visibleAtLaunch"
                    label="Show the overlay at launch"
                    @update:model-value="v => patch({ overlay: { visibleAtLaunch: v } })"
                />
            </SettingRow>
            <SettingRow v-if="openAtLogin !== null" label="Open at login">
                <Toggle :model-value="openAtLogin" label="Open at login" @update:model-value="setOpenAtLogin" />
            </SettingRow>
            <SettingRow label="Target new sessions" hint="A session you open takes over from the current target">
                <Toggle
                    :model-value="settings.sessions.targetNew"
                    label="Target new sessions"
                    @update:model-value="v => patch({ sessions: { targetNew: v } })"
                />
            </SettingRow>
        </Card>
    </template>

    <template v-if="settings && props.section === 'overlay'">
        <h1>Overlay</h1>
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
    </template>

    <template v-if="props.section === 'terminal'">
        <h1>Terminal command</h1>
        <Card>
            <SettingRow
                :label="cli.status.value?.command ?? 'cv'"
                :hint="cli.status.value?.installed ? (cli.status.value.upToDate ? 'Installed' : 'Installed, a reinstall is due') : 'Not installed'"
            >
                <button class="button" type="button" :disabled="cli.busy.value || cli.status.value?.foreign" @click="install">
                    {{ cli.status.value?.installed ? 'Reinstall' : 'Install' }}
                </button>
            </SettingRow>
            <p class="dim path"><code>{{ cli.status.value?.path }}</code></p>
            <p v-if="cli.status.value?.foreign" class="error-text">
                A <code>{{ cli.status.value.command }}</code> that Apollo didn't write is already there; remove or
                rename it to install.
            </p>
            <p v-if="cli.status.value && !cli.status.value.onPath" class="warn">
                <code>~/.local/bin</code> isn't on your <code>PATH</code>. Add
                <code>export PATH="$HOME/.local/bin:$PATH"</code> to your shell profile.
            </p>
            <p v-if="info && !stable" class="dim">
                <code>cv-dev</code> runs this checkout's plugin with the development flag, so Claude Code shows its
                development-channels dialog.
            </p>
            <SettingRow v-if="stable" label="Voice plugin" :hint="pluginHint" />
            <SettingRow v-if="stable && approval" label="Channel approval" :hint="approvalHints[approval]">
                <button v-if="approval !== 'approved' && approval !== 'conflict'" class="button" type="button" @click="copyApproval">
                    {{ approvalCopy.copied.value ? 'Copied' : 'Copy' }}
                </button>
            </SettingRow>
            <pre v-if="stable && approval && approval !== 'approved' && approval !== 'conflict'" class="command"><code>{{ cli.status.value?.approvalCommand }}</code></pre>
            <p v-if="cli.error.value" class="error-text">{{ cli.error.value }}</p>
        </Card>
    </template>

    <template v-if="stable && props.section === 'editors'">
        <h1>Cursor and VS Code panels</h1>
        <Card>
            <SettingRow label="cv-panel" :hint="idePanelHint" />
            <SettingRow v-for="editor in idePanel?.editors ?? []" :key="editor.name" :label="editor.name" :hint="editorHint(editor)">
                <button v-if="editor.wrapper !== 'ours'" class="button" type="button" @click="copySetting">
                    {{ settingCopy.copied.value ? 'Copied' : 'Copy' }}
                </button>
            </SettingRow>
            <pre v-if="idePanel?.editors.some(e => e.wrapper !== 'ours')" class="command"><code>{{ idePanel.setting }}</code></pre>
            <p v-if="idePanel && idePanel.editors.length === 0" class="dim">Neither Cursor nor VS Code is installed.</p>
        </Card>
    </template>

    <template v-if="settings && props.section === 'history'">
        <h1>History</h1>
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
    </template>

    <template v-if="props.section === 'about'">
        <h1>About</h1>
        <Card>
            <SettingRow label="Version"><span class="dim">{{ info?.version }}</span></SettingRow>
            <SettingRow label="Channel">
                <span class="dim">{{ info ? (stable ? 'Stable' : 'Dev build') : '' }}</span>
            </SettingRow>
            <SettingRow label="Commit"><span class="dim">{{ info?.commit }}</span></SettingRow>
            <Updates v-if="stable" />
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
    padding: 6px 8px;
    border: 0;
    border-radius: 8px;
    background: var(--control);
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
    border-radius: 8px;
    background: var(--control);
    font-size: 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
}
</style>
