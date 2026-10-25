import { readFile } from 'node:fs/promises'
import type { Discovery } from '../../../shared/types/Discovery'

function isDiscovery(value: unknown): value is Discovery {
    if (typeof value !== 'object' || value === null) return false
    const d = value as Record<string, unknown>
    return d.version === 1 && Number.isInteger(d.port) && typeof d.token === 'string' && /^[0-9a-f]{64}$/.test(d.token)
        && Number.isInteger(d.pid)
}

/** The hub's discovery file, or null while the app isn't running or the file is unreadable. */
export async function readDiscovery(path: string): Promise<Discovery | null> {
    try {
        const parsed: unknown = JSON.parse(await readFile(path, 'utf8'))
        return isDiscovery(parsed) ? parsed : null
    } catch {
        return null
    }
}
