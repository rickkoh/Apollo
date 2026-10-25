<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import * as api from '../../shared/api'
import StatusDot from '../../shared/components/StatusDot.vue'
import { useSessions } from '../../shared/composables/useSessions'
import MessageRow from '../components/MessageRow.vue'
import { useMessageFeed } from '../composables/useMessageFeed'

const PAGE = 50
const NEAR_EDGE_PX = 48

const props = defineProps<{ id: string }>()
const { sessions } = useSessions()
const session = computed(() => sessions.value.find(s => s.id === props.id) ?? null)
const { messages } = useMessageFeed(m => m.session === props.id)
const scroller = ref<HTMLElement | null>(null)
const loadingOlder = ref(false)
const exhausted = ref(false)

function markLatestRead(): void {
    const latest = [...messages.value].reverse().find(m => m.direction === 'from_session')
    if (latest) api.markRead(props.id, latest.id).catch(() => {})
}

async function scrollToBottom(): Promise<void> {
    await nextTick()
    if (scroller.value) scroller.value.scrollTop = scroller.value.scrollHeight
}

watch(() => props.id, async id => {
    exhausted.value = false
    messages.value = await api.history(id, PAGE)
    exhausted.value = messages.value.length < PAGE
    markLatestRead()
    await scrollToBottom()
}, { immediate: true })

watch(() => messages.value.length, async (now, before) => {
    const el = scroller.value
    if (!el || loadingOlder.value || now <= before) return
    if (el.scrollHeight - el.scrollTop - el.clientHeight < NEAR_EDGE_PX * 4) await scrollToBottom()
    markLatestRead()
})

async function onScroll(): Promise<void> {
    const el = scroller.value
    if (!el || el.scrollTop > NEAR_EDGE_PX || loadingOlder.value || exhausted.value || !messages.value[0]) return
    loadingOlder.value = true
    const before = el.scrollHeight
    const older = await api.history(props.id, PAGE, messages.value[0].id)
    exhausted.value = older.length < PAGE
    messages.value = [...older, ...messages.value]
    await nextTick()
    el.scrollTop = el.scrollHeight - before
    loadingOlder.value = false
}
</script>

<template>
    <div class="detail">
        <RouterLink to="/sessions" class="back">‹ Sessions</RouterLink>
        <h1>
            <StatusDot :state="session?.live ? 'live' : 'offline'" />
            {{ session?.title ?? props.id }}
        </h1>
        <p class="dim folder">{{ session?.cwd }}</p>
        <div ref="scroller" class="timeline" @scroll="onScroll">
            <p v-if="exhausted && messages.length > 0" class="dim edge">Beginning of the conversation</p>
            <p v-if="messages.length === 0" class="dim edge">No messages yet.</p>
            <MessageRow v-for="m in messages" :key="m.id" :message="m" />
        </div>
    </div>
</template>

<style scoped>
.detail {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 84px);
}
.back {
    color: var(--text-dim);
    text-decoration: none;
    font-size: 13px;
}
h1 {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 8px 0 2px;
}
.folder {
    margin: 0 0 16px;
}
.timeline {
    flex: 1;
    overflow-y: auto;
    padding: 0 16px;
    background: var(--card-bg);
    border: 1px solid var(--island-border);
    border-radius: var(--radius-card);
}
.edge {
    text-align: center;
    font-size: 12px;
}
</style>
