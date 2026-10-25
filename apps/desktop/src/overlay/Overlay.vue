<script setup lang="ts">
import { computed, onMounted, onScopeDispose, ref } from 'vue'
import * as api from '../shared/api'
import type { DotState } from '../shared/components/StatusDot.vue'
import { useHub } from '../shared/composables/useHub'
import { useListener } from '../shared/composables/useListener'
import { useSessions } from '../shared/composables/useSessions'
import { useSettings } from '../shared/composables/useSettings'
import type { SessionId, UiError } from '../shared/types'
import InputIsland from './components/InputIsland.vue'
import MessageStack from './components/MessageStack.vue'
import SessionPicker from './components/SessionPicker.vue'
import { useAutoResize } from './composables/useAutoResize'
import { useStack } from './composables/useStack'
import { useTarget } from './composables/useTarget'

const DELIVERED_TICK_MS = 1200

const root = ref<HTMLElement | null>(null)
const input = ref<InstanceType<typeof InputIsland> | null>(null)
const pickerOpen = ref(false)
const delivered = ref(false)
const dev = ref(false)
const command = ref('cv')
api.about().then(info => {
    dev.value = info.channel === 'dev'
    command.value = info.command
})

const hub = useHub()
const { sessions } = useSessions()
const { target, select } = useTarget(sessions)
const { stack, hovered } = useStack(target, text => input.value?.restore(text))
const { settings } = useSettings()
useAutoResize(root)

const status = computed<DotState>(() => {
    if (!hub.running.value) return 'hub-down'
    if (stack.pending) return 'pending'
    return target.value?.live ? 'live' : 'offline'
})
const otherUnread = computed(() => sessions.value.some(s => s.live && !s.isTarget && s.unread > 0))

let tickTimer: ReturnType<typeof setTimeout> | undefined
useListener(api.onMessageUpdated(update => {
    if (update.status !== 'delivered') return
    delivered.value = true
    clearTimeout(tickTimer)
    tickTimer = setTimeout(() => (delivered.value = false), DELIVERED_TICK_MS)
}))

function describe(err: unknown): string {
    if (!api.isUiError(err)) return String(err)
    const e: UiError = err
    switch (e.code) {
        case 'no_target':
            return 'Pick a session first'
        case 'target_offline':
            return `${target.value?.title ?? 'That session'} isn't connected`
        case 'too_large':
            return 'That message is too long to send'
        default:
            return e.message
    }
}

async function send(text: string): Promise<void> {
    stack.beginSend()
    try {
        stack.sent(await api.send(text))
    } catch (err) {
        if (api.isUiError(err) && err.code === 'empty') return
        hub.reportError(err)
        stack.error(describe(err))
        input.value?.restore(text)
    }
}

async function choose(id: SessionId): Promise<void> {
    pickerOpen.value = false
    try {
        await select(id)
    } catch (err) {
        hub.reportError(err)
        stack.error(describe(err))
    }
}

function onPointerDown(event: PointerEvent): void {
    if (pickerOpen.value && !(event.target as Element).closest('.picker, .chip')) pickerOpen.value = false
}
onMounted(() => document.addEventListener('pointerdown', onPointerDown))
onScopeDispose(() => document.removeEventListener('pointerdown', onPointerDown))

function openPanel(): void {
    const id = target.value?.id
    api.openPanel(id ? `/sessions/${encodeURIComponent(id)}` : undefined).catch(() => {})
}
</script>

<template>
    <main ref="root" class="overlay">
        <SessionPicker
            v-if="pickerOpen"
            class="picker"
            :sessions="sessions"
            :command="command"
            :target-id="target?.id ?? null"
            @select="choose"
            @close="pickerOpen = false"
        />
        <MessageStack
            v-else
            :cards="stack.cards"
            :preview-lines="settings?.overlay.replyPreviewLines ?? 3"
            @hover="hovered = $event"
            @open-panel="openPanel"
        />
        <InputIsland
            ref="input"
            :target="target"
            :status="status"
            :other-unread="otherUnread"
            :picker-open="pickerOpen"
            :delivered="delivered"
            :dev="dev"
            :command="command"
            @send="send"
            @toggle-picker="pickerOpen = !pickerOpen"
        />
    </main>
</template>

<style>
html,
body {
    background: transparent;
    overflow: hidden;
    font-size: 13px;
}
</style>

<style scoped>
.overlay {
    position: fixed;
    right: 0;
    bottom: 0;
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    padding: 12px;
}
</style>
