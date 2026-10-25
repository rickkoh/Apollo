import { ref } from 'vue'

const COPIED_MS = 1500

/** Copies text to the clipboard; `copied` reads true for a moment afterwards, for a "Copied" label. */
export function useCopy() {
    const copied = ref(false)
    let timer: ReturnType<typeof setTimeout> | undefined
    async function copy(text: string): Promise<void> {
        await navigator.clipboard.writeText(text)
        copied.value = true
        clearTimeout(timer)
        timer = setTimeout(() => (copied.value = false), COPIED_MS)
    }
    return { copied, copy }
}
