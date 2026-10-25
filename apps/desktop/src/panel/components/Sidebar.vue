<script setup lang="ts">
import { navigation } from '../router'
import Logo from './Logo.vue'
</script>

<template>
    <nav class="sidebar" aria-label="Main">
        <div class="brand" data-tauri-drag-region>
            <Logo />
            <span>Voice Bridge</span>
        </div>
        <RouterLink v-for="item in navigation" :key="item.path" :to="item.path" class="link">
            <span class="icon" aria-hidden="true">{{ item.icon }}</span>{{ item.label }}
        </RouterLink>
        <RouterLink to="/settings" class="link settings">
            <span class="icon" aria-hidden="true">⚙</span>Settings
        </RouterLink>
    </nav>
</template>

<style scoped>
.sidebar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 210px;
    flex: none;
    padding: 48px 12px 16px;
    border-right: 1px solid var(--island-border);
}
.brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px 18px;
    font-weight: 600;
}
.link {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: 8px;
    color: var(--text);
    text-decoration: none;
}
.link:hover {
    background: var(--hover);
}
.link.router-link-exact-active,
.link.router-link-active:not([href='#/']) {
    background: var(--card-bg);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.06);
    font-weight: 500;
}
.icon {
    width: 16px;
    text-align: center;
    color: var(--text-dim);
}
.settings {
    margin-top: auto;
}
</style>
