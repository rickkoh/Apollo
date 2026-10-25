<script setup lang="ts">
defineProps<{ label: string; hint?: string }>()
</script>

<template>
    <div class="row">
        <div class="text">
            <div>{{ label }}</div>
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
    padding: 12px 0;
    border-bottom: 1px solid var(--island-border);
}
.row:last-child {
    border-bottom: 0;
}
.hint {
    margin-top: 2px;
    font-size: 12px;
    color: var(--text-dim);
}
.control {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
}
</style>
