<script setup lang="ts">
import { useSettings } from '../shared/composables/useSettings'
import ClassicLayout from './layouts/ClassicLayout.vue'
import IslandLayout from './layouts/IslandLayout.vue'

const { settings } = useSettings()
</script>

<template>
    <IslandLayout v-if="settings.overlay.layout === 'island'" />
    <ClassicLayout v-else />
</template>

<style>
html,
body {
    background: transparent;
    overflow: hidden;
    font-size: 13px;
}
</style>
