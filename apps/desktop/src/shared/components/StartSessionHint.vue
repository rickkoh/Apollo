<script setup lang="ts">
import { START_SESSION } from '../copy'

const props = defineProps<{ command: string }>()
</script>

<template>
    <span>{{ START_SESSION.before }} <code>{{ props.command }}</code> {{ START_SESSION.after }}</span>
</template>
