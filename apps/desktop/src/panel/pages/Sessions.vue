<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useRouter } from 'vue-router'
import * as api from '../../shared/api'
import StatusDot from '../../shared/components/StatusDot.vue'
import { useSessions } from '../../shared/composables/useSessions'
import { relative } from '../../shared/format'
import type { SessionId, SessionView } from '../../shared/types'

const router = useRouter()
const { sessions } = useSessions()
const rows = computed(() => [...sessions.value].sort((a, b) => Number(b.live) - Number(a.live)))

const editing = ref<SessionId | null>(null)
const draft = ref('')
const confirming = ref<SessionId | null>(null)
const error = ref<string | null>(null)
const input = ref<HTMLInputElement[]>([])

async function run(action: () => Promise<void>): Promise<void> {
    error.value = null
    try {
        await action()
    } catch (err) {
        error.value = api.isUiError(err) ? err.message : String(err)
    }
}

async function startRename(s: SessionView): Promise<void> {
    editing.value = s.id
    draft.value = s.title
    await nextTick()
    input.value[0]?.select()
}

function saveRename(id: SessionId): Promise<void> {
    editing.value = null
    return run(() => api.renameSession(id, draft.value.trim() || null))
}

function makeTarget(id: SessionId): Promise<void> {
    return run(() => api.setTarget(id))
}

function forget(id: SessionId): Promise<void> {
    confirming.value = null
    return run(() => api.forgetSession(id))
}
</script>

<template>
    <h1>Sessions</h1>
    <p v-if="error" class="error-text">{{ error }}</p>
    <p v-if="rows.length === 0" class="dim">No sessions yet. Start one with <code>cv</code> in a terminal.</p>
    <table v-else>
        <thead>
            <tr>
                <th scope="col">Session</th>
                <th scope="col">Folder</th>
                <th scope="col">Last activity</th>
                <th scope="col">Unread</th>
                <th scope="col"><span class="visually-hidden">Actions</span></th>
            </tr>
        </thead>
        <tbody>
            <tr v-for="s in rows" :key="s.id" @click="router.push(`/sessions/${encodeURIComponent(s.id)}`)">
                <td class="title">
                    <StatusDot :state="s.live ? 'live' : 'offline'" />
                    <input
                        v-if="editing === s.id"
                        ref="input"
                        v-model="draft"
                        aria-label="Session name"
                        @click.stop
                        @keydown.enter="saveRename(s.id)"
                        @keydown.esc="editing = null"
                        @blur="saveRename(s.id)"
                    />
                    <span v-else>{{ s.title }}</span>
                    <span v-if="s.isTarget" class="badge">target</span>
                </td>
                <td class="dim folder" :title="s.cwd">{{ s.cwd }}</td>
                <td class="dim">{{ s.live ? 'live' : relative(s.lastActivity) }}</td>
                <td>{{ s.unread || '' }}</td>
                <td class="actions" @click.stop>
                    <button v-if="s.live && !s.isTarget" class="button" type="button" @click="makeTarget(s.id)">
                        Set as target
                    </button>
                    <button class="button" type="button" @click="startRename(s)">Rename</button>
                    <template v-if="!s.live">
                        <button v-if="confirming === s.id" class="button danger" type="button" @click="forget(s.id)">
                            Forget history?
                        </button>
                        <button v-else class="button" type="button" @click="confirming = s.id">Forget</button>
                    </template>
                </td>
            </tr>
        </tbody>
    </table>
</template>

<style scoped>
table {
    width: 100%;
    border-collapse: collapse;
    background: var(--card-bg);
    border: 1px solid var(--island-border);
    border-radius: var(--radius-card);
    overflow: hidden;
}
th {
    padding: 10px 14px;
    text-align: left;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
    border-bottom: 1px solid var(--island-border);
}
td {
    padding: 10px 14px;
    border-bottom: 1px solid var(--island-border);
}
tbody tr {
    cursor: pointer;
}
tbody tr:hover {
    background: var(--hover);
}
tbody tr:last-child td {
    border-bottom: 0;
}
.title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
}
.folder {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
}
.badge {
    padding: 1px 6px;
    border-radius: 6px;
    background: var(--hover);
    font-size: 11px;
    font-weight: 400;
    color: var(--text-dim);
}
.actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    white-space: nowrap;
}
.visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
}
input {
    font: inherit;
    padding: 2px 6px;
    border: 1px solid var(--island-border);
    border-radius: 6px;
    background: var(--panel-bg);
    color: var(--text);
}
</style>
