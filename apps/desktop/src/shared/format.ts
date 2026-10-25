const time = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' })
const day = new Intl.DateTimeFormat(undefined, { weekday: 'long', day: 'numeric', month: 'long' })

export function clock(iso: string): string {
    return time.format(new Date(iso))
}

export function dayLabel(iso: string, now = new Date()): string {
    const date = new Date(iso)
    const startOf = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
    const days = Math.round((startOf(now) - startOf(date)) / 86_400_000)
    if (days === 0) return 'Today'
    if (days === 1) return 'Yesterday'
    return day.format(date)
}

export function relative(iso: string | null, now = Date.now()): string {
    if (!iso) return 'never'
    const minutes = Math.round((now - new Date(iso).getTime()) / 60_000)
    if (minutes < 1) return 'just now'
    if (minutes < 60) return `${minutes} min ago`
    const hours = Math.round(minutes / 60)
    if (hours < 24) return `${hours} h ago`
    return dayLabel(iso, new Date(now))
}

export function withinHours(iso: string | null, hours: number, now = Date.now()): boolean {
    return iso !== null && now - new Date(iso).getTime() <= hours * 3_600_000
}

export function truncate(text: string, max: number): string {
    return text.length <= max ? text : `${text.slice(0, max - 1)}…`
}
