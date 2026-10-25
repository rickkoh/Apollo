<script setup lang="ts">
defineProps<{ label: string; hint?: string }>()
</script>

<template>
    <div class="row">
        <div class="text">
            <div class="label">{{ label }}</div>
            <div v-if="hint" class="hint">{{ hint }}</div>
        </div>
        <div class="control"><slot /></div>
    </div>
</template>

<style scoped>
.row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 16px 0;
    border-bottom: 1px solid var(--island-border);
}
.row:first-child {
    padding-top: 4px;
}
.row:last-child {
    padding-bottom: 4px;
    border-bottom: 0;
}
.label {
    font-weight: 500;
}
.hint {
    margin-top: 3px;
    font-size: 13px;
    color: var(--text-dim);
    overflow-wrap: anywhere;
}
.control {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
}
</style>
