<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from 'vue'
import type { Card } from '../stack'

const props = defineProps<{ card: Card; previewLines: number }>()
const emit = defineEmits<{ 'update:expanded': [expanded: boolean]; 'open-panel': [] }>()

const body = ref<HTMLElement | null>(null)
const overflowing = ref(false)
const maxExpanded = `${Math.round(window.screen.availHeight * 0.4)}px`

async function measure(): Promise<void> {
    await nextTick()
    overflowing.value = (body.value?.scrollHeight ?? 0) > (body.value?.clientHeight ?? 0) + 1
}
onMounted(measure)
watch(() => props.previewLines, measure)

function toggle(): void {
    if (overflowing.value || props.card.expanded) emit('update:expanded', !props.card.expanded)
}

const labels = { sent: 'Your message', reply: 'Reply from Claude', error: 'Error' } as const
</script>

<template>
    <section
        class="island"
        :class="props.card.kind"
        :aria-label="labels[props.card.kind]"
        @click="toggle"
        @dblclick="emit('open-panel')"
    >
        <p
            ref="body"
            class="body"
            :class="{ expanded: props.card.expanded }"
            :style="props.card.expanded ? { maxHeight: maxExpanded } : { WebkitLineClamp: props.previewLines }"
        >
            <span v-if="props.card.kind === 'sent'" class="you">You: </span>{{ props.card.text }}
        </p>
        <button
            v-if="overflowing && !props.card.expanded"
            class="more"
            type="button"
            @click.stop="emit('update:expanded', true)"
        >
            more
        </button>
    </section>
</template>

<style scoped>
.island {
    position: relative;
    width: calc(100% - 24px);
    padding: 10px 14px;
    background: var(--island-bg);
    border: 1px solid var(--island-border);
    border-radius: var(--radius-reply);
    box-shadow: var(--island-shadow);
    cursor: default;
    user-select: text;
}
.sent {
    color: var(--text-dim);
}
.error {
    color: var(--error);
}
.you {
    font-weight: 500;
}
.body {
    margin: 0;
    line-height: 18px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    overflow: hidden;
}
.body.expanded {
    display: block;
    overflow-y: auto;
}
.more {
    position: absolute;
    right: 10px;
    bottom: 8px;
    padding: 0 4px 0 16px;
    border: 0;
    background: linear-gradient(to right, transparent, var(--island-bg) 14px);
    color: var(--accent);
    cursor: pointer;
}
</style>
