<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useRouter } from 'vue-router'
import * as api from '../../shared/api'
import Icon from '../../shared/components/Icon.vue'
import StartSessionHint from '../../shared/components/StartSessionHint.vue'
import StatusDot from '../../shared/components/StatusDot.vue'
import { useSessions } from '../../shared/composables/useSessions'
import { homeRelative, relative } from '../../shared/format'
import type { SessionId, SessionView } from '../../shared/types'
import Card from '../components/Card.vue'

const router = useRouter()
const { sessions } = useSessions()
const rows = computed(() => [...sessions.value].sort((a, b) => Number(b.live) - Number(a.live)))
const command = ref('cv')
api.about().then(info => (command.value = info.command))
const home = ref<string | null>(null)
api.homeDir().then(dir => (home.value = dir))

const editing = ref<SessionId | null>(null)
const draft = ref('')
const confirming = ref<SessionId | null>(null)
const error = ref<string | null>(null)
const input = ref<HTMLInputElement[]>([])

const detail = (id: SessionId) => `/sessions/${encodeURIComponent(id)}`

async function run(action: () => Promise<void>): Promise<void> {
    error.value = null
    try {
        await action()
    } catch (err) {
        error.value = api.errorMessage(err)
    }
}

async function startRename(s: SessionView): Promise<void> {
    editing.value = s.id
    draft.value = s.title
    await nextTick()
    input.value[0]?.select()
}

async function saveRename(s: SessionView): Promise<void> {
    editing.value = null
    const title = draft.value.trim()
    if (title === s.title) return
    await run(() => api.renameSession(s.id, title || null))
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
    <Card v-if="rows.length === 0">
        <p class="dim empty"><StartSessionHint :command="command" /></p>
    </Card>
    <div v-else class="table">
        <table>
            <colgroup>
                <col class="session" />
                <col />
                <col class="activity" />
                <col class="actions" />
            </colgroup>
            <thead>
                <tr>
                    <th scope="col">Session</th>
                    <th scope="col">Folder</th>
                    <th scope="col">Last activity</th>
                    <th scope="col"><span class="visually-hidden">Actions</span></th>
                </tr>
            </thead>
            <tbody>
                <tr v-for="s in rows" :key="s.id" @click="router.push(detail(s.id))">
                    <td>
                        <div class="title">
                            <StatusDot :state="s.live ? 'live' : 'offline'" />
                            <input
                                v-if="editing === s.id"
                                ref="input"
                                v-model="draft"
                                aria-label="Session name"
                                @click.stop
                                @keydown.enter="saveRename(s)"
                                @keydown.esc="editing = null"
                                @blur="saveRename(s)"
                            />
                            <RouterLink v-else :to="detail(s.id)" class="name" :title="s.title" @click.stop>{{ s.title }}</RouterLink>
                            <span v-if="s.unread" class="pill" :title="`${s.unread} unread`">{{ s.unread }}</span>
                            <span v-if="s.isTarget" class="badge">Target</span>
                        </div>
                    </td>
                    <td class="dim">
                        <div class="folder" :title="s.cwd"><bdi>{{ homeRelative(s.cwd, home) }}</bdi></div>
                    </td>
                    <td class="dim">
                        <template v-if="s.live">{{ s.activity === 'busy' ? 'Working' : 'Live' }}</template>
                        <template v-else>{{ relative(s.lastActivity) }}</template>
                    </td>
                    <td @click.stop>
                        <div v-if="confirming === s.id" class="confirm">
                            <button class="button danger small" type="button" @click="forget(s.id)">Forget</button>
                            <button class="icon-button" type="button" aria-label="Keep" title="Keep" @click="confirming = null">
                                <Icon name="close" />
                            </button>
                        </div>
                        <div v-else class="row-actions">
                            <button
                                v-if="s.live && !s.isTarget"
                                class="icon-button slot-target"
                                type="button"
                                aria-label="Set as target"
                                title="Set as target"
                                @click="makeTarget(s.id)"
                            >
                                <Icon name="target" />
                            </button>
                            <button
                                class="icon-button slot-rename"
                                type="button"
                                aria-label="Rename"
                                title="Rename"
                                @click="startRename(s)"
                            >
                                <Icon name="rename" />
                            </button>
                            <button
                                v-if="!s.live"
                                class="icon-button slot-forget"
                                type="button"
                                aria-label="Forget history"
                                title="Forget history"
                                @click="confirming = s.id"
                            >
                                <Icon name="forget" />
                            </button>
                        </div>
                    </td>
                </tr>
            </tbody>
        </table>
    </div>
</template>

<style scoped>
.table {
    background: var(--card-bg);
    border: 1px solid var(--island-border);
    border-radius: var(--radius-card);
    overflow: hidden;
}
table {
    width: 100%;
    table-layout: fixed;
    border-collapse: separate;
    border-spacing: 0;
}
col.session {
    width: 38%;
}
col.activity {
    width: 104px;
}
col.actions {
    width: 116px;
}
th {
    padding: 12px 14px;
    text-align: left;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    color: var(--text-dim);
}
td {
    height: 52px;
    padding: 0 14px;
    border-top: 1px solid var(--island-border);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
tbody tr {
    cursor: pointer;
    transition: background var(--motion);
}
tbody tr:hover {
    background: var(--hover);
}
.title {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
}
.name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
    color: var(--text);
    text-decoration: none;
}
.badge {
    flex: none;
    padding: 2px 7px;
    border-radius: 6px;
    background: var(--control);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-dim);
}
.folder {
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
}
.pill {
    flex: none;
    min-width: 20px;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--accent);
    color: #fff;
    font-size: 11px;
    font-weight: 600;
    text-align: center;
}
.row-actions {
    display: grid;
    grid-template-columns: repeat(3, 28px);
    justify-content: end;
    gap: 2px;
}
.slot-target {
    grid-column: 1;
}
.slot-rename {
    grid-column: 2;
}
.slot-forget {
    grid-column: 3;
}
.confirm {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 2px;
}
.button.small {
    padding: 5px 10px;
    font-size: 13px;
}
input {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
    border: 1px solid var(--island-border);
    border-radius: 6px;
    background: var(--surface);
}
.empty {
    margin: 0;
}
</style>
