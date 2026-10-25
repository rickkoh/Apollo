<script setup lang="ts">
import { computed, ref } from 'vue'
import * as api from '../../shared/api'
import { useCli } from '../../shared/composables/useCli'
import { useSessions } from '../../shared/composables/useSessions'
import { dayLabel } from '../../shared/format'
import type { Message } from '../../shared/types'
import Card from '../components/Card.vue'
import MessageRow from '../components/MessageRow.vue'
import Toggle from '../components/Toggle.vue'
import { useMessageFeed } from '../composables/useMessageFeed'
import { useOverlayVisible } from '../composables/useOverlayVisible'

const RECENT = 30

const { sessions, live } = useSessions()
const overlay = useOverlayVisible()
const cli = useCli()
const { messages } = useMessageFeed(() => true, RECENT)
api.recentMessages(RECENT).then(list => (messages.value = list))

const greeting = (() => {
    const hour = new Date().getHours()
    return hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening'
})()
const target = computed(() => sessions.value.find(s => s.isTarget) ?? null)
const titles = computed(() => new Map(sessions.value.map(s => [s.id, s.title])))
const groups = computed(() => {
    const out: { day: string; messages: Message[] }[] = []
    for (const message of [...messages.value].reverse()) {
        const day = dayLabel(message.at)
        if (out.at(-1)?.day !== day) out.push({ day, messages: [] })
        out.at(-1)!.messages.push(message)
    }
    return out
})

const copied = ref(false)
async function copyCommand(): Promise<void> {
    await navigator.clipboard.writeText('cv')
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
}
</script>

<template>
    <h1>{{ greeting }}</h1>
    <div class="cards">
        <Card title="Overlay">
            <div class="stat">
                <span>{{ overlay.visible.value ? 'On' : 'Off' }}</span>
                <Toggle :model-value="overlay.visible.value" label="Show the overlay" @update:model-value="overlay.set" />
            </div>
        </Card>
        <Card title="Live sessions">
            <div class="stat">{{ live.length }}</div>
        </Card>
        <Card title="Target">
            <div class="stat target">
                {{ target?.title ?? 'None' }}
                <span v-if="target && !target.live" class="dim small">not connected</span>
            </div>
        </Card>
    </div>

    <h2>Start a voice session</h2>
    <div v-if="cli.status.value?.installed" class="command">
        <code>$ cv</code>
        <button class="button" type="button" @click="copyCommand">{{ copied ? 'Copied' : 'Copy' }}</button>
    </div>
    <div v-else class="command">
        <span class="dim">Install the <code>cv</code> command to start voice sessions from a terminal.</span>
        <button class="button" type="button" :disabled="cli.busy.value" @click="cli.install">Install</button>
    </div>
    <p v-if="cli.error.value" class="error-text">{{ cli.error.value }}</p>

    <h2>Recent</h2>
    <p v-if="groups.length === 0" class="dim">Nothing yet. Messages you dictate and Claude's replies show up here.</p>
    <section v-for="group in groups" :key="group.day">
        <h3 class="day">{{ group.day }}</h3>
        <MessageRow
            v-for="m in group.messages"
            :key="m.id"
            :message="m"
            :session-title="titles.get(m.session) ?? m.session.slice(0, 8)"
        />
    </section>
</template>

<style scoped>
.cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
}
.stat {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 20px;
    font-weight: 600;
}
.target {
    flex-direction: column;
    align-items: flex-start;
    font-size: 16px;
    overflow-wrap: anywhere;
}
.small {
    font-size: 12px;
    font-weight: 400;
}
.command {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 16px;
    background: var(--card-bg);
    border: 1px solid var(--island-border);
    border-radius: var(--radius-card);
}
.day {
    margin: 16px 0 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
}
</style>
