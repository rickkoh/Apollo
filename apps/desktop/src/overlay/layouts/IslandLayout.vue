<script setup lang="ts">
import { computed, onMounted, onScopeDispose, ref, watch } from 'vue'
import { useSettings } from '../../shared/composables/useSettings'
import InputIsland from '../components/InputIsland.vue'
import MessageStack from '../components/MessageStack.vue'
import NotchRow, { type Signal } from '../components/NotchRow.vue'
import SessionPicker from '../components/SessionPicker.vue'
import { useElementSize, useFrame, type Size } from '../composables/useFrame'
import { useGeometry } from '../composables/useGeometry'
import { useIsland } from '../composables/useIsland'
import { useOverlay } from '../composables/useOverlay'

const WING = 36
const SIDE_MARGIN = 16
const SHADOW_MARGIN = 24
const OPEN_MS = 380
const CLOSE_MS = 240

const input = ref<InstanceType<typeof InputIsland> | null>(null)
const body = ref<HTMLElement | null>(null)
const { settings } = useSettings()
const { sessions, target, hub, stack, status, otherUnread, delivered, dev, command, pickerOpen, send, choose, openPanel } =
    useOverlay({
        fadeMs: () => null,
        restore: text => input.value?.restore(text),
        onReply: () => reply(),
        onError: () => error(),
    })

const busy = computed(() => target.value?.activity === 'busy')
const failing = computed(() => !hub.running.value || stack.cards.some(c => c.kind === 'error'))
const { island, escape, reply, error } = useIsland({
    news: () => busy.value || failing.value || stack.pending,
    peekMs: () => settings.value.overlay.replyFadeSeconds * 1000,
    onOpen: () => input.value?.focus(),
})
const signal = computed<Signal | null>(() => {
    if (failing.value) return 'error'
    if (stack.pending) return 'pending'
    return island.unseen ? 'unseen' : null
})
const expanded = computed(() => island.state === 'open' || island.state === 'peek')
watch(expanded, open => {
    if (!open) pickerOpen.value = false
})

const geometry = useGeometry()
const notch = computed<Size>(() => ({
    width: geometry.value?.notchWidth ?? 0,
    height: geometry.value?.notchHeight ?? 0,
}))
const mirrored = computed(() => geometry.value !== null && geometry.value.anchor !== 'top')
const margin = computed(() => ({ left: mirrored.value ? 0 : SIDE_MARGIN, right: SIDE_MARGIN }))
const openWidth = computed(() => settings.value.overlay.width - margin.value.left - margin.value.right)
const bodySize = useElementSize(body)

const shape = computed<Size>(() => {
    if (expanded.value) return { width: openWidth.value, height: notch.value.height + bodySize.value.height }
    const wings = island.state === 'compact' ? 2 * WING : 0
    return { width: notch.value.width + wings, height: notch.value.height }
})
const wanted = computed<Size>(() => {
    if (!expanded.value) return shape.value
    return {
        width: shape.value.width + margin.value.left + margin.value.right,
        height: shape.value.height + SHADOW_MARGIN,
    }
})

// The window grows before a morph and shrinks once it ends, so the shape never clips.
const frame = ref<Size>({ width: 0, height: 0 })
const growing = ref(false)
const settled = ref(true)
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)')
let settleTimer: ReturnType<typeof setTimeout> | undefined
watch(
    wanted,
    (next, previous) => {
        growing.value = !previous || next.width > previous.width || next.height > previous.height
        frame.value = {
            width: Math.max(frame.value.width, next.width),
            height: Math.max(frame.value.height, next.height),
        }
        settled.value = false
        clearTimeout(settleTimer)
        const morph = reducedMotion.matches ? 0 : (growing.value ? OPEN_MS : CLOSE_MS) + 40
        settleTimer = setTimeout(() => {
            frame.value = wanted.value
            settled.value = true
        }, morph)
    },
    { immediate: true },
)
onScopeDispose(() => clearTimeout(settleTimer))
useFrame(() => frame.value)

const rest = computed(() => island.state === 'hidden' && settled.value && geometry.value?.hardwareNotch === true)

const px = (n: number) => `${n}px`
const bodyStyle = computed(() => ({
    width: px(openWidth.value),
    [mirrored.value ? 'bottom' : 'top']: px(notch.value.height),
}))

function onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape' && !pickerOpen.value) escape()
}
onMounted(() => document.addEventListener('keydown', onKey, true))
onScopeDispose(() => document.removeEventListener('keydown', onKey, true))
</script>

<template>
    <main class="island-tokens frame" :class="{ mirrored }" :style="{ width: px(frame.width), height: px(frame.height) }">
        <div
            class="shape"
            :class="[island.state, { growing, rest, expanded }]"
            :style="{ width: px(shape.width), height: px(shape.height) }"
        >
            <div class="clip">
                <div class="notch" :style="{ height: px(notch.height) }">
                    <NotchRow
                        :notch-width="notch.width"
                        :full="expanded"
                        :status="status"
                        :signal="signal"
                        :busy="busy"
                        :delivered="delivered"
                        :other-unread="otherUnread"
                        :dev="dev"
                    />
                </div>
                <div ref="body" class="body" :style="bodyStyle">
                    <InputIsland
                        ref="input"
                        inset
                        :target="target"
                        :status="status"
                        :other-unread="otherUnread"
                        :picker-open="pickerOpen"
                        :delivered="delivered"
                        :dev="dev"
                        :command="command"
                        :max-lines="settings.overlay.inputLines"
                        @send="send"
                        @toggle-picker="pickerOpen = !pickerOpen"
                    />
                    <SessionPicker
                        v-if="pickerOpen"
                        class="picker"
                        inset
                        :sessions="sessions"
                        :command="command"
                        :target-id="target?.id ?? null"
                        :ended-hours="settings.overlay.pickerEndedHours"
                        @select="choose"
                        @close="pickerOpen = false"
                    />
                    <MessageStack
                        v-else
                        inset
                        :cards="stack.cards"
                        :preview-lines="settings.overlay.replyPreviewLines"
                        :expanded-height-percent="settings.overlay.expandedHeightPercent"
                        @open-panel="openPanel"
                    />
                </div>
            </div>
        </div>
    </main>
</template>

<style scoped>
.frame {
    position: fixed;
    top: 0;
    left: 0;
    overflow: hidden;
}
.shape {
    --t: 240ms;
    --ease: cubic-bezier(0.4, 0, 0.2, 1);
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    border-radius: 0 0 10px 10px;
    transition:
        width var(--t) var(--ease),
        height var(--t) var(--ease),
        border-radius var(--t) var(--ease),
        box-shadow var(--t);
}
.shape.growing {
    --t: 380ms;
    --ease: cubic-bezier(0.34, 1.36, 0.64, 1);
}
.shape.compact {
    border-radius: 0 0 14px 14px;
}
.shape.expanded {
    border-radius: 0 0 var(--radius-island) var(--radius-island);
    box-shadow: var(--island-shadow);
}
/* Concave shoulders where the island meets the top edge. */
.shape::before,
.shape::after {
    content: '';
    position: absolute;
    top: 0;
    width: 10px;
    height: 10px;
    opacity: 0;
    transition: opacity var(--t);
}
.shape::before {
    left: -10px;
    background: radial-gradient(circle at 0 100%, transparent 9.5px, var(--island-bg) 10px);
}
.shape::after {
    right: -10px;
    background: radial-gradient(circle at 100% 100%, transparent 9.5px, var(--island-bg) 10px);
}
.shape.expanded::before,
.shape.expanded::after {
    opacity: 1;
}
.clip {
    position: absolute;
    inset: 0;
    overflow: hidden;
    border-radius: inherit;
    background: var(--island-bg);
}
.shape.rest .clip {
    background: transparent;
}
.notch {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
}
.body {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 10px 10px;
    opacity: 0;
    transition: opacity 120ms ease;
}
.shape.expanded .body {
    opacity: 1;
    transition-delay: 80ms;
}
.mirrored .shape {
    top: auto;
    bottom: 0;
    left: 0;
    transform: none;
    border-radius: 10px 10px 0 0;
}
.mirrored .shape.compact {
    border-radius: 14px 14px 0 0;
}
.mirrored .shape.expanded {
    border-radius: var(--radius-island) var(--radius-island) 0 0;
}
.mirrored .shape::before,
.mirrored .shape::after {
    display: none;
}
.mirrored .notch {
    top: auto;
    bottom: 0;
}
.mirrored .body {
    left: 0;
    transform: none;
    flex-direction: column-reverse;
    padding: 10px 10px 0;
}
.mirrored .body :deep(.stack) {
    flex-direction: column-reverse;
}
@media (prefers-reduced-motion: reduce) {
    .shape,
    .shape::before,
    .shape::after,
    .body {
        transition: none;
    }
}
</style>
