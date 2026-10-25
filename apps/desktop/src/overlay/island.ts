export type IslandState = 'hidden' | 'compact' | 'peek' | 'open'

export const DWELL_MS = 150
export const GRACE_MS = 300
/** How long a summoned island waits for a send once the shortcut's key comes up. */
export const SUMMON_IDLE_MS = 5000

/**
 * The island's state from the pointer, the panel's keyboard, the focus shortcut and arriving messages. The caller owns the clock: it
 * passes `now` (ms) in and calls `advance(now)` once `due` has passed.
 */
export class Island {
    /** Claude working, a message waiting or an error: the wings show while the island is closed. */
    news = false
    /** A reply arrived since the island was last open. */
    unseen = false
    private mode: 'closed' | 'peek' | 'open' = 'closed'
    private inside = false
    private dismissed = false
    /** Opened by the shortcut: the pointer leaving doesn't close it, a send does. */
    private summoned = false
    private openAt: number | null = null
    private closeAt: number | null = null
    private peekUntil: number | null = null

    get state(): IslandState {
        if (this.mode !== 'closed') return this.mode
        return this.news || this.unseen ? 'compact' : 'hidden'
    }

    get due(): number | null {
        const times = [this.openAt, this.closeAt, this.peekUntil].filter(t => t !== null)
        return times.length > 0 ? Math.min(...times) : null
    }

    /** The shell reports only real crossings, so an enter means the pointer left since any Esc. */
    pointer(inside: boolean, now: number): void {
        this.inside = inside
        this.dismissed = false
        if (inside) {
            this.summoned = false
            this.closeAt = null
            if (this.mode !== 'open') this.openAt = now + DWELL_MS
        } else {
            this.openAt = null
            if (this.mode === 'open' && !this.summoned) this.closeAt = now + GRACE_MS
        }
    }

    /** The panel gained the keyboard (a click) or lost it (⌘-Tab, the control panel). */
    keyboard(has: boolean, now: number): void {
        if (has && this.mode !== 'open') {
            this.open()
            if (!this.inside) this.closeAt = now + GRACE_MS
        } else if (!has && this.mode === 'open') {
            this.dismiss()
        }
    }

    /** The focus shortcut fired. */
    summon(): void {
        this.open()
        this.summoned = true
    }

    /** The shortcut's held key came up. */
    released(now: number): void {
        if (this.summoned && this.mode === 'open') this.closeAt = now + SUMMON_IDLE_MS
    }

    sent(): void {
        if (this.summoned) this.close()
    }

    escape(): void {
        if (this.mode === 'open') this.dismiss()
    }

    reply(now: number, peekMs: number): void {
        if (this.mode === 'open') return
        this.unseen = true
        this.peek(now, peekMs)
    }

    error(now: number, peekMs: number): void {
        if (this.mode !== 'open') this.peek(now, peekMs)
    }

    advance(now: number): void {
        if (this.openAt !== null && now >= this.openAt) {
            this.openAt = null
            if (this.inside && !this.dismissed) this.open()
        }
        if (this.closeAt !== null && now >= this.closeAt) {
            this.closeAt = null
            if (!this.inside) this.close()
        }
        if (this.peekUntil !== null && now >= this.peekUntil) {
            this.peekUntil = null
            if (this.mode === 'peek') this.close()
        }
    }

    private peek(now: number, peekMs: number): void {
        this.mode = 'peek'
        this.peekUntil = now + peekMs
    }

    private open(): void {
        this.mode = 'open'
        this.unseen = false
        this.openAt = null
        this.closeAt = null
        this.peekUntil = null
    }

    private close(): void {
        this.mode = 'closed'
        this.summoned = false
        this.closeAt = null
        this.peekUntil = null
    }

    /** Closes until the pointer leaves and comes back. */
    private dismiss(): void {
        this.close()
        this.openAt = null
        this.dismissed = this.inside
    }
}
