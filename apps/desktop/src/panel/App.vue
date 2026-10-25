<script setup lang="ts">
import { useRouter } from 'vue-router'
import * as api from '../shared/api'
import { useListener } from '../shared/composables/useListener'
import Sidebar from './components/Sidebar.vue'

const router = useRouter()
useListener(api.onPanelNavigate(route => router.push(route)))
</script>

<template>
    <div class="panel">
        <div class="drag" data-tauri-drag-region />
        <Sidebar />
        <main class="content">
            <RouterView />
        </main>
    </div>
</template>

<style>
html,
body {
    height: 100%;
    background: var(--panel-bg);
    font-size: 14px;
}
#app {
    height: 100%;
}
h1 {
    margin: 0 0 20px;
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.01em;
}
h2 {
    margin: 28px 0 10px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
}
.button {
    padding: 6px 12px;
    border: 1px solid var(--island-border);
    border-radius: 8px;
    background: var(--card-bg);
    cursor: pointer;
}
.button:hover:not(:disabled) {
    background: var(--hover);
}
.button.danger {
    color: var(--error);
}
.button:disabled {
    opacity: 0.5;
    cursor: default;
}
.dim {
    color: var(--text-dim);
}
.error-text {
    color: var(--error);
}
code {
    font-family: ui-monospace, 'SF Mono', Menlo, monospace;
    font-size: 0.92em;
}
</style>

<style scoped>
.panel {
    position: relative;
    display: flex;
    height: 100%;
}
.content {
    position: relative;
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    margin-top: 36px;
    padding: 8px 40px 40px;
}
.drag {
    position: absolute;
    inset: 0 0 auto;
    height: 36px;
}
</style>
