<script setup lang="ts">
import { ref } from 'vue'
import * as api from '../../shared/api'
import { useListener } from '../../shared/composables/useListener'
import { useSettings } from '../../shared/composables/useSettings'
import InputIsland from '../components/InputIsland.vue'
import MessageStack from '../components/MessageStack.vue'
import SessionPicker from '../components/SessionPicker.vue'
import { useElementSize, useFrame } from '../composables/useFrame'
import { useOverlay } from '../composables/useOverlay'

const root = ref<HTMLElement | null>(null)
const input = ref<InstanceType<typeof InputIsland> | null>(null)
const { settings } = useSettings()
const { sessions, target, stack, hovered, status, otherUnread, delivered, dev, command, pickerOpen, send, choose, openPanel } =
    useOverlay({
        fadeMs: () => settings.value.overlay.replyFadeSeconds * 1000,
        restore: text => input.value?.restore(text),
    })

useListener(api.onOverlaySummon(({ released }) => {
    if (released) return
    pickerOpen.value = false
    api.overlayKeyboard(true).then(() => input.value?.focus(), () => {})
}))

const size = useElementSize(root)
useFrame(() => ({ width: settings.value.overlay.width, height: size.value.height }))
</script>

<template>
    <main ref="root" class="overlay" :style="{ width: `${settings.overlay.width}px` }">
        <SessionPicker
            v-if="pickerOpen"
            class="picker"
            :sessions="sessions"
            :command="command"
            :target-id="target?.id ?? null"
            :ended-hours="settings.overlay.pickerEndedHours"
            @select="choose"
            @close="pickerOpen = false"
        />
        <MessageStack
            v-else
            :cards="stack.cards"
            :preview-lines="settings.overlay.replyPreviewLines"
            :expanded-height-percent="settings.overlay.expandedHeightPercent"
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
            :max-lines="settings.overlay.inputLines"
            @send="send"
            @toggle-picker="pickerOpen = !pickerOpen"
        />
    </main>
</template>

<style scoped>
.overlay {
    position: fixed;
    right: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    padding: 12px;
}
</style>
