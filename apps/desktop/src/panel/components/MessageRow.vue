<script setup lang="ts">
import { computed } from 'vue'
import { clock } from '../../shared/format'
import type { Message } from '../../shared/types'

const props = defineProps<{ message: Message; sessionTitle?: string }>()

const outgoing = computed(() => props.message.direction === 'to_session')
const who = computed(() => {
    if (props.sessionTitle === undefined) return outgoing.value ? 'You' : 'Claude'
    return outgoing.value ? `you → ${props.sessionTitle}` : props.sessionTitle
})
const statusText = computed(() => {
    switch (props.message.status) {
        case 'queued':
            return 'sending…'
        case 'delivered':
            return 'delivered'
        case 'failed':
            return `not delivered${props.message.detail ? `: ${props.message.detail}` : ''}`
        default:
            return ''
    }
})
</script>

<template>
    <div class="row" :class="{ outgoing }">
        <span class="time">{{ clock(props.message.at) }}</span>
        <span class="who">{{ who }}</span>
        <span class="text">{{ props.message.text }}</span>
        <span v-if="outgoing && statusText" class="status" :class="props.message.status">{{ statusText }}</span>
    </div>
</template>

<style scoped>
.row {
    display: grid;
    grid-template-columns: 48px 150px 1fr auto;
    gap: 12px;
    align-items: baseline;
    padding: 8px 0;
    border-bottom: 1px solid var(--island-border);
}
.time,
.who {
    color: var(--text-dim);
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
}
.status {
    font-size: 12px;
    color: var(--text-dim);
}
.status.failed {
    color: var(--error);
}
.status.queued {
    color: var(--warn);
}
</style>
