<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import StatusDot, { type DotState } from '../../shared/components/StatusDot.vue'
import type { SessionView } from '../../shared/types'
import { usePasteToSend } from '../composables/usePasteToSend'

const props = defineProps<{
    target: SessionView | null
    status: DotState
    otherUnread: boolean
    pickerOpen: boolean
    delivered: boolean
    dev: boolean
    command: string
}>()
const emit = defineEmits<{ send: [text: string]; 'toggle-picker': [] }>()

const box = ref<HTMLTextAreaElement | null>(null)
const text = ref('')
const MAX_LINES = 3

const disabled = computed(() => props.status === 'hub-down' || !props.target?.live)
const placeholder = computed(() => {
    if (props.status === 'hub-down') return "Apollo isn't running its hub. Restart the app"
    if (!props.target?.live) return `Start a session with ${props.command} in any terminal, Cursor's included`
    return 'Speak or type…'
})

function grow(): void {
    const el = box.value
    if (!el) return
    el.style.height = 'auto'
    const line = parseFloat(getComputedStyle(el).lineHeight)
    el.style.height = `${Math.min(el.scrollHeight, line * MAX_LINES)}px`
}
watch(text, () => nextTick(grow))

function sendBox(): void {
    if (!text.value.trim() || disabled.value) return
    emit('send', text.value)
    text.value = ''
}

usePasteToSend(box, sent => {
    if (sent === text.value) sendBox()
    else if (!disabled.value) emit('send', sent)
})

defineExpose({
    restore(previous: string): void {
        if (!text.value.trim()) text.value = previous
    },
})
</script>

<template>
    <div class="island" role="group" aria-label="Voice input">
        <button
            class="chip"
            type="button"
            :aria-expanded="props.pickerOpen"
            aria-haspopup="listbox"
            :title="props.target?.cwd ?? 'No session'"
            @click="emit('toggle-picker')"
        >
            <StatusDot :state="props.status" />
            <span v-if="props.dev" class="dev" title="Apollo Dev">DEV</span>
            <span class="title">{{ props.target?.title ?? 'No session' }}</span>
            <span v-if="props.target?.activity === 'busy'" class="activity">· working</span>
            <span v-if="props.otherUnread" class="unread" aria-label="Another session has unread replies" />
            <span class="caret" aria-hidden="true">{{ props.pickerOpen ? '▴' : '▾' }}</span>
        </button>
        <span class="divider" aria-hidden="true" />
        <textarea
            ref="box"
            v-model="text"
            rows="1"
            aria-label="Message to Claude"
            :placeholder="placeholder"
            :disabled="disabled"
        />
        <button class="send" type="button" :disabled="disabled || !text.trim()" aria-label="Send" @click="sendBox">
            <span v-if="props.delivered" class="tick" aria-label="Delivered">✓</span>
            <span v-else aria-hidden="true">⏎</span>
        </button>
    </div>
</template>

<style scoped>
.island {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-height: 44px;
    padding: 6px 8px 6px 12px;
    background: var(--island-bg);
    border: 1px solid var(--island-border);
    border-radius: var(--radius-input);
    box-shadow: var(--island-shadow);
}
.chip {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 200px;
    padding: 4px 6px;
    border: 0;
    border-radius: 12px;
    background: transparent;
    cursor: pointer;
}
.chip:hover,
.chip:focus-visible {
    background: var(--hover);
}
.title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
}
.dev {
    flex: none;
    padding: 0 5px;
    border-radius: 6px;
    background: var(--warn);
    color: #18181b;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
}
.activity {
    flex: none;
    color: var(--text-dim);
    white-space: nowrap;
}
.unread {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    flex: none;
}
.caret {
    color: var(--text-dim);
    font-size: 10px;
}
.divider {
    width: 1px;
    align-self: stretch;
    margin: 6px 2px;
    background: var(--island-border);
}
textarea {
    flex: 1;
    min-width: 0;
    resize: none;
    border: 0;
    outline: none;
    background: transparent;
    font: inherit;
    line-height: 18px;
    color: var(--text);
    padding: 4px 0;
}
textarea::placeholder {
    color: var(--text-dim);
}
.send {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
}
.send:not(:disabled):hover {
    background: var(--hover);
    color: var(--text);
}
.tick {
    color: var(--ok);
}
</style>
