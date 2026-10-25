import { onMounted, onScopeDispose, type Ref } from 'vue'

/**
 * D3: a paste sends the clipboard text at once and cancels the web view's own insertion, which comes up empty
 * from Wispr Flow's second dictation on. Enter sends the box; Shift+Enter adds a line.
 */
export function usePasteToSend(
    box: Ref<HTMLTextAreaElement | null>,
    send: { paste: (text: string) => void; enter: () => void },
): void {
    function onPaste(event: ClipboardEvent): void {
        event.preventDefault()
        const text = event.clipboardData?.getData('text/plain') ?? ''
        if (text.trim()) send.paste(text)
    }

    function onKeydown(event: KeyboardEvent): void {
        if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return
        event.preventDefault()
        send.enter()
    }

    onMounted(() => {
        box.value?.addEventListener('paste', onPaste)
        box.value?.addEventListener('keydown', onKeydown)
    })
    onScopeDispose(() => {
        box.value?.removeEventListener('paste', onPaste)
        box.value?.removeEventListener('keydown', onKeydown)
    })
}
