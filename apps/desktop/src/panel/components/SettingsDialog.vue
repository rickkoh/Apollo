<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import * as api from '../../shared/api'
import Icon, { type IconName } from '../../shared/components/Icon.vue'
import type { AboutInfo } from '../../shared/types'
import Settings from '../pages/Settings.vue'

const props = defineProps<{ section: string }>()
const router = useRouter()

const info = ref<AboutInfo | null>(null)
api.about().then(a => (info.value = a))

type Item = { id: string; label: string; icon: IconName }
const groups = computed<{ label: string; items: Item[] }[]>(() => [
    {
        label: 'Settings',
        items: [
            { id: 'general', label: 'General', icon: 'general' },
            { id: 'overlay', label: 'Overlay', icon: 'overlay' },
            { id: 'terminal', label: 'Terminal', icon: 'terminal' },
            ...(info.value?.channel === 'stable' ? [{ id: 'editors', label: 'Editors', icon: 'editors' } as const] : []),
            { id: 'history', label: 'History', icon: 'history' },
        ],
    },
    { label: 'Apollo', items: [{ id: 'about', label: 'About', icon: 'about' }] },
])
const active = computed(() => {
    const known = groups.value.some(g => g.items.some(i => i.id === props.section))
    return known || !info.value ? props.section : 'general'
})

const dialog = ref<HTMLDialogElement | null>(null)
onMounted(() => {
    dialog.value?.showModal()
    dialog.value?.focus()
})

const show = (section: string) => router.replace({ query: { settings: section } })
const close = () => router.replace({ query: {} })

let pressedBackdrop = false
const onPointerDown = (event: PointerEvent) => (pressedBackdrop = event.target === dialog.value)
function onClick(event: MouseEvent): void {
    if (pressedBackdrop && event.target === dialog.value) close()
}
</script>

<template>
    <dialog
        ref="dialog"
        class="dialog"
        tabindex="-1"
        aria-label="Settings"
        @cancel.prevent="close"
        @pointerdown="onPointerDown"
        @click="onClick"
    >
        <div class="frame">
            <nav class="nav" aria-label="Settings sections">
                <template v-for="group in groups" :key="group.label">
                    <div class="group">{{ group.label }}</div>
                    <button
                        v-for="item in group.items"
                        :key="item.id"
                        type="button"
                        class="item"
                        :aria-current="item.id === active ? 'page' : undefined"
                        @click="show(item.id)"
                    >
                        <Icon :name="item.icon" />{{ item.label }}
                    </button>
                </template>
                <div v-if="info" class="version">
                    Apollo v{{ info.version }}<template v-if="info.channel === 'dev'"> · Dev</template>
                </div>
            </nav>
            <div class="body">
                <button type="button" class="icon-button close" aria-label="Close settings" title="Close" @click="close">
                    <Icon name="close" />
                </button>
                <div class="scroll">
                    <Settings :section="active" />
                </div>
            </div>
        </div>
    </dialog>
</template>

<style scoped>
.dialog {
    width: min(820px, calc(100vw - 96px));
    height: min(560px, calc(100vh - 96px));
    max-width: none;
    max-height: none;
    padding: 0;
    border: 1px solid var(--island-border);
    border-radius: 16px;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--dialog-shadow);
    overflow: hidden;
    animation: fade-in var(--motion) ease-out;
}
.dialog:focus {
    outline: none;
}
.dialog::backdrop {
    background: var(--backdrop, rgb(0 0 0 / 0.4));
    animation: fade-in var(--motion) ease-out;
}
@keyframes fade-in {
    from {
        opacity: 0;
    }
}
.frame {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100%;
}
.nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 12px;
    border-right: 1px solid var(--island-border);
    background: var(--panel-bg);
}
.group {
    padding: 14px 10px 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
}
.group:first-child {
    padding-top: 4px;
}
.item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    text-align: left;
    cursor: pointer;
    transition: background var(--motion);
}
.item:hover {
    background: var(--hover);
}
.item .icon {
    color: var(--text-dim);
}
.item[aria-current='page'] {
    background: var(--control);
    font-weight: 500;
}
.item[aria-current='page'] .icon {
    color: var(--text);
}
.version {
    margin-top: auto;
    padding: 8px 10px 0;
    font-size: 12px;
    color: var(--text-dim);
}
.body {
    position: relative;
    min-width: 0;
    min-height: 0;
}
.close {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 1;
}
.scroll {
    height: 100%;
    overflow-y: auto;
    padding: 36px 40px 40px;
}
</style>
