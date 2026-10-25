import type { Readable, Writable } from 'node:stream'

/** Enables the installed, user-disabled voice plugin for this one session. */
export const PLUGIN_SETTINGS = '{"enabledPlugins":{"voice@voice-bridge":true}}'

const SERVER = 'plugin:voice:voice'
const PREFIX = 'voice-bridge-'

export interface BridgeOptions {
    retryMs?: number
    maxAttempts?: number
    log?: (message: string) => void
}

/** Calls `onLine` with each complete line of `stream`, without its newline, and with a last unterminated one. */
export function eachLine(stream: Readable, onLine: (line: string) => void): void {
    let buffer = ''
    stream.setEncoding('utf8')
    stream.on('data', (chunk: string) => {
        buffer += chunk
        let end
        while ((end = buffer.indexOf('\n')) !== -1) {
            const line = buffer.slice(0, end)
            buffer = buffer.slice(end + 1)
            onLine(line)
        }
    })
    stream.on('end', () => {
        if (buffer) onLine(buffer)
        buffer = ''
    })
}

function requestId(line: string, subtype: string): string | null {
    if (!line.includes(`"subtype":"${subtype}"`)) return null
    try {
        const id = JSON.parse(line).request_id
        return typeof id === 'string' ? id : null
    } catch {
        return null
    }
}

function controlResponse(line: string): { request_id?: unknown; subtype?: string; error?: string } | null {
    if (!line.startsWith('{"type":"control_response"')) return null
    try {
        return JSON.parse(line).response ?? null
    } catch {
        return null
    }
}

/** Relays an IDE panel and its claude unchanged, and enables the channel the panel never enables (PLAN.md §24). */
export function bridge(
    host: { input: Readable; output: Writable },
    claude: { stdin: Writable; stdout: Readable },
    { retryMs = 1000, maxAttempts = 30, log = () => {} }: BridgeOptions = {},
): void {
    let initialize: string | null = null
    let attempts = 0
    const toClaude = (line: string) => {
        if (claude.stdin.writable) claude.stdin.write(line + '\n')
    }

    const enable = () => {
        attempts += 1
        toClaude(JSON.stringify({
            type: 'control_request',
            request_id: `${PREFIX}${attempts}`,
            request: { subtype: 'channel_enable', serverName: SERVER },
        }))
    }

    eachLine(host.input, line => {
        initialize ??= requestId(line, 'initialize')
        toClaude(line)
    })
    host.input.on('end', () => claude.stdin.end())

    eachLine(claude.stdout, line => {
        const response = controlResponse(line)
        if (response && response.request_id === initialize && attempts === 0) {
            enable()
        } else if (response && typeof response.request_id === 'string' && response.request_id.startsWith(PREFIX)) {
            if (response.subtype === 'success') {
                log('voice channel enabled')
            } else if (/not connected/.test(response.error ?? '') && attempts < maxAttempts) {
                setTimeout(enable, retryMs)
            } else {
                log(`cannot enable the voice channel: ${response.error ?? 'no reason given'}`)
            }
            return
        }
        host.output.write(line + '\n')
    })
}
