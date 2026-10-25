<script setup lang="ts">
import { computed, onMounted, onScopeDispose } from 'vue'
import StartSessionHint from '../../shared/components/StartSessionHint.vue'
import StatusDot from '../../shared/components/StatusDot.vue'
import { clock, withinHours } from '../../shared/format'
import type { SessionId, SessionView } from '../../shared/types'

const props = defineProps<{
    sessions: SessionView[]
    targetId: SessionId | null
    command: string
    endedHours: number
    inset?: boolean
}>()
const emit = defineEmits<{ select: [id: SessionId]; close: [] }>()

const rows = computed(() => [
    ...props.sessions.filter(s => s.live),
    ...props.sessions.filter(s => !s.live && props.endedHours > 0 && withinHours(s.lastActivity, props.endedHours)),
])

function onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') emit('close')
}
onMounted(() => document.addEventListener('keydown', onKey))
onScopeDispose(() => document.removeEventListener('keydown', onKey))
</script>

<template>
    <ul class="island" :class="{ inset: props.inset }" role="listbox" aria-label="Sessions">
        <li v-if="rows.length === 0" class="empty"><StartSessionHint :command="props.command" /></li>
        <li v-for="s in rows" :key="s.id" role="option" :aria-selected="s.id === props.targetId" :aria-disabled="!s.live">
            <button type="button" :disabled="!s.live" :title="s.cwd" @click="emit('select', s.id)">
                <StatusDot :state="s.live ? 'live' : 'offline'" />
                <span class="title">{{ s.title }}</span>
                <span v-if="s.unread > 0 && s.id !== props.targetId" class="unread" aria-label="Unread replies" />
                <span class="meta">
                    <template v-if="s.id === props.targetId">✓</template>
                    <template v-else-if="!s.live && s.lastActivity">ended {{ clock(s.lastActivity) }}</template>
                </span>
            </button>
        </li>
    </ul>
</template>

<style scoped>
.island {
    width: 100%;
    margin: 0;
    padding: 6px;
    list-style: none;
    background: var(--island-bg);
    border: 1px solid var(--island-border);
    border-radius: var(--radius-reply);
    box-shadow: var(--island-shadow);
}
.island.inset {
    padding: 2px 0;
    background: transparent;
    border: 0;
    box-shadow: none;
}
button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    text-align: left;
    cursor: pointer;
}
button:not(:disabled):hover,
button:focus-visible {
    background: var(--hover);
}
button:disabled {
    color: var(--text-dim);
    cursor: default;
}
.title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.unread {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
}
.meta {
    color: var(--text-dim);
    font-size: 12px;
}
.empty {
    padding: 8px 10px;
    color: var(--text-dim);
}
</style>
