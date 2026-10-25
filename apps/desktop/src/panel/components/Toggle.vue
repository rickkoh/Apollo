<script setup lang="ts">
const props = defineProps<{ modelValue: boolean; label: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
    <button
        type="button"
        role="switch"
        class="toggle"
        :aria-checked="props.modelValue"
        :aria-label="props.label"
        @click="emit('update:modelValue', !props.modelValue)"
    >
        <span class="knob" />
    </button>
</template>

<style scoped>
.toggle {
    position: relative;
    width: 36px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 11px;
    background: var(--idle);
    cursor: pointer;
    transition: background var(--motion);
}
.toggle[aria-checked='true'] {
    background: var(--ok);
}
.knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    transition: transform var(--motion);
}
.toggle[aria-checked='true'] .knob {
    transform: translateX(14px);
}
</style>
