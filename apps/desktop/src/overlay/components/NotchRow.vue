<script setup lang="ts">
import StatusDot, { type DotState } from '../../shared/components/StatusDot.vue'

export type Signal = 'error' | 'pending' | 'unseen'

const props = defineProps<{
    notchWidth: number
    /** Open or peeking; otherwise the row is the wings either side of the notch. */
    full: boolean
    status: DotState
    signal: Signal | null
    busy: boolean
    delivered: boolean
    otherUnread: boolean
    dev: boolean
}>()

const signalLabel: Record<Signal, string> = {
    error: 'Something went wrong',
    pending: 'Waiting for delivery',
    unseen: 'A new reply',
}
</script>

<template>
    <div class="row" :class="{ full: props.full }" :style="{ gridTemplateColumns: `1fr ${props.notchWidth}px 1fr` }">
        <div class="side left">
            <template v-if="props.full">
                <StatusDot :state="props.status" />
                <span v-if="props.dev" class="dev" title="Apollo Dev">DEV</span>
            </template>
            <span
                v-else-if="props.signal"
                class="signal"
                :class="props.signal"
                role="img"
                :aria-label="signalLabel[props.signal]"
            />
        </div>
        <span aria-hidden="true" />
        <div class="side right">
            <span v-if="props.busy" class="working" role="status" aria-label="Claude is working">
                <span class="pulse" aria-hidden="true"><i /><i /><i /></span>
                <span v-if="props.full" class="label">working</span>
            </span>
            <template v-if="props.full">
                <span v-if="props.delivered" class="tick" aria-label="Delivered">✓</span>
                <span v-if="props.otherUnread" class="unread" aria-label="Another session has unread replies" />
            </template>
        </div>
    </div>
</template>

<style scoped>
.row {
    display: grid;
    align-items: center;
    height: 100%;
}
.side {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
    justify-content: center;
}
.full .left {
    justify-content: flex-start;
    padding-left: 12px;
}
.full .right {
    justify-content: flex-end;
    padding-right: 12px;
}
.dev {
    padding: 0 5px;
    border-radius: 6px;
    background: var(--warn);
    color: #18181b;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
}
.signal {
    width: 8px;
    height: 8px;
    border-radius: 50%;
}
.signal.error {
    background: var(--error);
}
.signal.pending {
    background: var(--warn);
}
.signal.unseen {
    background: var(--accent);
}
/* Right to left on one fixed-height line: a label without room wraps out of sight instead of being cut. */
.working {
    display: flex;
    flex-flow: row-reverse wrap;
    align-content: flex-start;
    align-items: center;
    column-gap: 5px;
    min-width: 0;
    height: 16px;
    overflow: hidden;
    color: var(--text-dim);
    font-size: 12px;
    white-space: nowrap;
}
.pulse {
    display: flex;
    gap: 3px;
}
.pulse i {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--text);
    animation: pulse 1.2s ease-in-out infinite;
}
.pulse i:nth-child(2) {
    animation-delay: 0.15s;
}
.pulse i:nth-child(3) {
    animation-delay: 0.3s;
}
@keyframes pulse {
    0%,
    80%,
    100% {
        opacity: 0.25;
    }
    40% {
        opacity: 1;
    }
}
@media (prefers-reduced-motion: reduce) {
    .pulse i {
        animation: none;
        opacity: 0.6;
    }
}
.tick {
    color: var(--ok);
}
.unread {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
}
</style>
