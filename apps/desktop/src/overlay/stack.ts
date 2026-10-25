import type { DeliveryStatus, Message, MessageId } from '../shared/types'

export type CardKind = 'sent' | 'reply' | 'error'

export interface Card {
    key: string
    kind: CardKind
    text: string
    /** The stored message behind a `sent` or `reply` card. */
    message: Message | null
    /** Milliseconds left before the card leaves; null while it waits (a pending echo, an error). */
    ttl: number | null
    expanded: boolean
}

export const MAX_CARDS = 3
export const FADE_MS = 20_000

/**
 * The overlay's timeline of the target's messages, newest last. Replies aren't paired with the messages they answer,
 * so a reply starts the fade of every echo before it.
 */
export class MessageStack {
    cards: Card[] = []

    clear(): void {
        this.cards = []
    }

    /** A new send clears errors, which otherwise stay. */
    beginSend(): void {
        this.cards = this.cards.filter(c => c.kind !== 'error')
    }

    /** The same message arrives twice (event and send result); the first copy is never older, so keep it. */
    sent(message: Message): void {
        const key = `m${message.id}`
        if (this.cards.some(c => c.key === key)) return
        this.push({ key, kind: 'sent', text: message.text, message, ttl: null, expanded: false })
    }

    /** Returns the sent text when the message failed, so the caller can put it back in the box. */
    updated(id: MessageId, status: DeliveryStatus, detail: string | null): string | null {
        const card = this.cards.find(c => c.key === `m${id}`)
        if (!card?.message) return null
        card.message = { ...card.message, status, detail }
        if (status !== 'failed') return null
        card.kind = 'error'
        card.text = `Not delivered: ${detail ?? 'unknown reason'}`
        card.ttl = null
        return card.message.text
    }

    reply(message: Message): void {
        for (const card of this.cards) {
            if (card.kind === 'sent' && card.ttl === null) card.ttl = FADE_MS
        }
        this.push({ key: `m${message.id}`, kind: 'reply', text: message.text, message, ttl: FADE_MS, expanded: false })
    }

    error(text: string): void {
        this.push({ key: `e${Date.now()}-${this.cards.length}`, kind: 'error', text, message: null, ttl: null, expanded: false })
    }

    /** Counts fading cards down by `elapsedMs`, except while the stack is hovered or a card is expanded. */
    tick(elapsedMs: number, paused: boolean): void {
        if (paused) return
        for (const card of this.cards) {
            if (card.ttl !== null && !card.expanded) card.ttl -= elapsedMs
        }
        this.cards = this.cards.filter(c => c.ttl === null || c.ttl > 0)
    }

    get pending(): boolean {
        return this.cards.some(c => c.kind === 'sent' && c.message?.status === 'queued')
    }

    private push(card: Card): void {
        this.cards.push(card)
        while (this.cards.length > MAX_CARDS) this.cards.shift()
    }
}
