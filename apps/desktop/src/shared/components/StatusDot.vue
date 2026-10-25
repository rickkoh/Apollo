<script setup lang="ts">
export type DotState = 'live' | 'pending' | 'offline' | 'hub-down'

const props = defineProps<{ state: DotState }>()

const label: Record<DotState, string> = {
    live: 'Connected',
    pending: 'Waiting for delivery',
    offline: 'No live session',
    'hub-down': 'Hub not running',
}
</script>

<template>
    <span class="dot" :class="props.state" role="img" :aria-label="label[props.state]" :title="label[props.state]" />
</template>

<style scoped>
.dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
    background: var(--idle);
    transition: background var(--motion);
}
.live {
    background: var(--ok);
}
.pending {
    background: var(--warn);
}
.hub-down {
    background: var(--error);
}
</style>
