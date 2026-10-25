<script setup lang="ts">
import { ref } from 'vue'
import { useRoute } from 'vue-router'
import * as api from '../../shared/api'
import Icon from '../../shared/components/Icon.vue'
import { navigation } from '../router'
import Logo from './Logo.vue'

const props = defineProps<{ collapsed: boolean }>()
const route = useRoute()
const active = (path: string) => route.path === path || (path !== '/' && route.path.startsWith(`${path}/`))

const dev = ref(false)
api.about().then(info => (dev.value = info.channel === 'dev'))
</script>

<template>
    <nav class="sidebar" :class="{ collapsed: props.collapsed }" aria-label="Main">
        <div class="brand" data-tauri-drag-region>
            <Logo />
            <span :class="{ 'visually-hidden': props.collapsed }">Apollo</span>
            <span v-if="dev" class="badge" :class="{ 'visually-hidden': props.collapsed }">Dev</span>
        </div>
        <RouterLink
            v-for="item in navigation"
            :key="item.path"
            :to="item.path"
            class="link"
            :class="{ active: active(item.path) }"
            :title="props.collapsed ? item.label : undefined"
        >
            <Icon :name="item.icon" /><span :class="{ 'visually-hidden': props.collapsed }">{{ item.label }}</span>
        </RouterLink>
        <RouterLink to="/settings" class="link settings" :title="props.collapsed ? 'Settings' : undefined">
            <Icon name="settings" /><span :class="{ 'visually-hidden': props.collapsed }">Settings</span>
        </RouterLink>
    </nav>
</template>

<style scoped>
.sidebar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 208px;
    padding: 4px 12px 16px;
}
.sidebar.collapsed {
    width: 60px;
}
.brand {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    margin-bottom: 20px;
    padding: 0 7px;
    font-size: 16px;
    font-weight: 600;
}
.brand svg {
    flex: none;
}
.badge {
    padding: 1px 6px;
    border-radius: 6px;
    background: var(--control);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-dim);
}
.link {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 10px;
    border-radius: 8px;
    color: var(--text);
    text-decoration: none;
    transition: background var(--motion);
}
.link:hover {
    background: var(--hover);
}
.link .icon {
    color: var(--text-dim);
}
.link.active {
    background: var(--control);
    font-weight: 500;
}
.link.active .icon {
    color: var(--text);
}
.settings {
    margin-top: auto;
}
</style>
