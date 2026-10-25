<script setup lang="ts">
defineProps<{ title?: string }>()
</script>

<template>
    <section class="card">
        <div v-if="title" class="title">{{ title }}</div>
        <slot />
    </section>
</template>

<style scoped>
.card {
    padding: 16px 20px;
    background: var(--card-bg);
    border: 1px solid var(--island-border);
    border-radius: var(--radius-card);
}
:slotted(:last-child) {
    margin-bottom: 0;
}
.title {
    margin-bottom: 8px;
    font-size: 12px;
    color: var(--text-dim);
}
</style>
