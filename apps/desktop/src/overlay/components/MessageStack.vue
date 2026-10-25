<script setup lang="ts">
import type { Card } from '../stack'
import ReplyIsland from './ReplyIsland.vue'

const props = defineProps<{ cards: Card[]; previewLines: number }>()
const emit = defineEmits<{ hover: [hovered: boolean]; 'open-panel': [] }>()
</script>

<template>
    <TransitionGroup
        tag="div"
        name="card"
        class="stack"
        aria-live="polite"
        @mouseenter="emit('hover', true)"
        @mouseleave="emit('hover', false)"
    >
        <ReplyIsland
            v-for="card in props.cards"
            :key="card.key"
            v-model:expanded="card.expanded"
            :card="card"
            :preview-lines="props.previewLines"
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
.card-move,
.card-enter-active,
.card-leave-active {
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
</style>
