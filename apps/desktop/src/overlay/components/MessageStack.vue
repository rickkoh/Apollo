<script setup lang="ts">
import { computed } from 'vue'
import type { Card } from '../stack'
import ReplyIsland from './ReplyIsland.vue'

const props = defineProps<{
    cards: Card[]
    previewLines: number
    expandedHeightPercent: number
    /** Rows inside the island, newest first. */
    inset?: boolean
}>()
const emit = defineEmits<{ hover: [hovered: boolean]; 'open-panel': [] }>()

const ordered = computed(() => (props.inset ? [...props.cards].reverse() : props.cards))
</script>

<template>
    <TransitionGroup
        tag="div"
        :name="props.inset ? 'row' : 'card'"
        class="stack"
        :class="{ inset: props.inset }"
        aria-live="polite"
        @mouseenter="emit('hover', true)"
        @mouseleave="emit('hover', false)"
    >
        <ReplyIsland
            v-for="card in ordered"
            :key="card.key"
            v-model:expanded="card.expanded"
            :card="card"
            :preview-lines="props.previewLines"
            :expanded-height-percent="props.expandedHeightPercent"
            :inset="props.inset"
            @open-panel="emit('open-panel')"
        />
    </TransitionGroup>
</template>

<style scoped>
.stack {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    width: 100%;
}
.stack.inset {
    align-items: stretch;
    gap: 2px;
}
.card-move,
.card-enter-active,
.card-leave-active,
.row-move,
.row-enter-active,
.row-leave-active {
    transition:
        transform var(--motion) ease,
        opacity var(--motion) ease;
}
.card-enter-from {
    opacity: 0;
    transform: translateY(12px);
}
.card-leave-to {
    opacity: 0;
}
.card-leave-active {
    position: absolute;
    right: 0;
}
.row-enter-from {
    opacity: 0;
    transform: translateY(-6px);
}
.row-leave-to {
    opacity: 0;
}
.row-leave-active {
    position: absolute;
    left: 0;
    right: 0;
}
</style>
