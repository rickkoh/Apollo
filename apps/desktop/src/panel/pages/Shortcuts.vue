<script setup lang="ts">
import { ref } from 'vue'
import * as api from '../../shared/api'
import { useSettings } from '../../shared/composables/useSettings'
import type { ModifierKey, ShortcutSettings } from '../../shared/types'
import Card from '../components/Card.vue'
import SettingRow from '../components/SettingRow.vue'

const { settings } = useSettings()
const error = ref<string | null>(null)

const shortcuts: { key: keyof ShortcutSettings; label: string; hint: string }[] = [
    { key: 'focusInput', label: 'Focus the voice box', hint: 'Shows the overlay and puts the cursor in the text box' },
    { key: 'toggleOverlay', label: 'Show or hide the overlay', hint: 'Toggles the overlay without the menu bar' },
]

const keys: { value: ModifierKey; label: string }[] = [
    { value: 'rightOption', label: 'Right ⌥ Option' },
    { value: 'leftOption', label: 'Left ⌥ Option' },
    { value: 'rightCommand', label: 'Right ⌘ Command' },
    { value: 'leftCommand', label: 'Left ⌘ Command' },
    { value: 'rightControl', label: 'Right ⌃ Control' },
    { value: 'leftControl', label: 'Left ⌃ Control' },
]

async function bind(shortcut: keyof ShortcutSettings, e: Event): Promise<void> {
    const value = (e.target as HTMLSelectElement).value
    error.value = null
    try {
        settings.value = await api.updateSettings({ shortcuts: { [shortcut]: value ? (value as ModifierKey) : null } })
    } catch (err) {
        error.value = api.errorMessage(err)
    }
}
</script>

<template>
    <h1>Shortcuts</h1>
    <p class="dim">Double-tap a modifier key from any app. Hold the second tap to keep dictating.</p>
    <p v-if="error" class="error-text" role="alert">{{ error }}</p>
    <Card>
        <SettingRow v-for="s in shortcuts" :key="s.key" :label="s.label" :hint="s.hint">
            <select :aria-label="`${s.label} shortcut`" :value="settings.shortcuts[s.key] ?? ''" @change="e => bind(s.key, e)">
                <option value="">Off</option>
                <option v-for="k in keys" :key="k.value" :value="k.value">Double-tap {{ k.label }}</option>
            </select>
        </SettingRow>
    </Card>
</template>

<style scoped>
.dim {
    color: var(--text-dim);
}
select {
    padding: 6px 8px;
    border: 0;
    border-radius: 8px;
    background: var(--control);
    font: inherit;
    color: var(--text);
}
</style>
