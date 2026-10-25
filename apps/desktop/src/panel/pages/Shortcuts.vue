<script setup lang="ts">
import Card from '../components/Card.vue'
import SettingRow from '../components/SettingRow.vue'

const shortcuts = [
    { label: 'Focus the voice box', hint: 'Jump to the overlay from any app' },
    { label: 'Show or hide the overlay', hint: 'Toggle the overlay without the menu bar' },
]
</script>

<template>
    <h1>Shortcuts</h1>
    <Card>
        <SettingRow v-for="s in shortcuts" :key="s.label" :label="s.label" :hint="s.hint">
            <input class="key" disabled placeholder="Coming soon" :aria-label="`${s.label} shortcut`" />
        </SettingRow>
    </Card>
</template>

<style scoped>
.key {
    width: 140px;
    padding: 6px 10px;
    border: 1px dashed var(--island-border);
    border-radius: 8px;
    background: transparent;
    font: inherit;
    color: var(--text-dim);
    text-align: center;
}
</style>
