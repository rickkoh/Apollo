<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import * as api from '../shared/api'
import Icon from '../shared/components/Icon.vue'
import { useListener } from '../shared/composables/useListener'
import SettingsDialog from './components/SettingsDialog.vue'
import Sidebar from './components/Sidebar.vue'

const router = useRouter()
const route = useRoute()
useListener(api.onPanelNavigate(target => router.push(target)))

const collapsed = ref(false)
</script>

<template>
    <div class="panel" :class="{ collapsed }">
        <header class="titlebar" data-tauri-drag-region>
            <button
                type="button"
                class="icon-button"
                :aria-label="collapsed ? 'Show the sidebar' : 'Hide the sidebar'"
                :title="collapsed ? 'Show the sidebar' : 'Hide the sidebar'"
                @click="collapsed = !collapsed"
            >
                <Icon name="sidebar" />
            </button>
        </header>
        <Sidebar :collapsed="collapsed" />
        <main class="sheet">
            <div class="scroll">
                <div class="page" :class="{ fill: route.meta.fill }">
                    <RouterView />
                </div>
            </div>
        </main>
        <SettingsDialog v-if="typeof route.query.settings === 'string'" :section="route.query.settings" />
    </div>
</template>

<style>
html,
body {
    height: 100%;
    background: var(--panel-bg);
    font-size: 14px;
    -webkit-font-smoothing: antialiased;
}
#app {
    height: 100%;
}
h1 {
    margin: 0 0 24px;
    font-family: var(--font-display);
    font-size: 28px;
    font-weight: 500;
    letter-spacing: -0.01em;
}
h2 {
    margin: 32px 0 12px;
    font-size: 15px;
    font-weight: 600;
}
p {
    line-height: 1.45;
}
.button {
    padding: 7px 14px;
    border: 0;
    border-radius: 8px;
    background: var(--control);
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    transition: background var(--motion);
}
.button:hover:not(:disabled) {
    background: var(--control-hover);
}
.button.danger {
    color: var(--error);
}
.button:disabled {
    opacity: 0.5;
    cursor: default;
}
.icon-button {
    display: inline-grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: background var(--motion), color var(--motion);
}
.icon-button:hover {
    background: var(--control);
    color: var(--text);
}
:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
}
input {
    font: inherit;
    color: var(--text);
}
.dim {
    color: var(--text-dim);
}
.error-text {
    color: var(--error);
}
.visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
}
code {
    font-family: ui-monospace, 'SF Mono', Menlo, monospace;
    font-size: 0.92em;
}
</style>

<style scoped>
.panel {
    display: grid;
    grid-template: 44px 1fr / auto 1fr;
    height: 100%;
}
.titlebar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    padding-left: 84px;
}
.sheet {
    min-width: 0;
    min-height: 0;
    margin: 0 8px 8px 0;
    border: 1px solid var(--island-border);
    border-radius: var(--radius-sheet);
    background: var(--surface);
    overflow: hidden;
}
.scroll {
    height: 100%;
    overflow-y: auto;
    padding: 36px 40px 40px;
}
.page {
    max-width: 880px;
    margin: 0 auto;
}
.page.fill {
    height: 100%;
}
</style>
