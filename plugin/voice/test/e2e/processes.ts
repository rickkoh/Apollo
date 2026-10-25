import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createInterface } from 'node:readline'
import { fileURLToPath } from 'node:url'

export const ROOT = fileURLToPath(new URL('../../../../', import.meta.url))
const HUB_DEV = `${ROOT}target/debug/hub-dev`
const CHANNEL = `${ROOT}plugin/voice/dist/channel.js`

async function poll<T>(probe: () => T | undefined | Promise<T | undefined>, timeoutMs: number, what: string): Promise<T> {
    const end = Date.now() + timeoutMs
    for (;;) {
        const value = await probe()
        if (value !== undefined) return value
        if (Date.now() > end) throw new Error(`timed out waiting for ${what}`)
        await new Promise(resolve => setTimeout(resolve, 50))
    }
}

export interface SessionRow {
    id: string
    title: string
    live: boolean
    target: boolean
}

export class HubDev {
    readonly lines: string[] = []
    readonly proc: ChildProcessWithoutNullStreams

    constructor(dataDir: string, discoveryPath: string) {
        this.proc = spawn(HUB_DEV, ['--data', dataDir, '--discovery', discoveryPath], { env: { ...process.env, RUST_LOG: 'warn' } })
        createInterface({ input: this.proc.stdout }).on('line', line => this.lines.push(line))
    }

    async ready(): Promise<void> {
        await poll(() => (this.lines.some(l => l.startsWith('hub-dev listening')) ? true : undefined), 10000, 'hub-dev to listen')
    }

    run(command: string): void {
        this.proc.stdin.write(`${command}\n`)
    }

    async waitLine(pattern: RegExp, timeoutMs = 5000): Promise<string> {
        return poll(() => this.lines.find(l => pattern.test(l)), timeoutMs, `a hub-dev line matching ${pattern}`)
    }

    async sessions(): Promise<SessionRow[]> {
        const start = this.lines.length
        this.run('sessions')
        await new Promise(resolve => setTimeout(resolve, 100))
        return this.lines.slice(start).flatMap(line => {
            const m = /^(\S+) (.+) \[(live|ended)\] \S+ unread=\d+( \*target\*)?$/.exec(line)
            return m ? [{ id: m[1]!, title: m[2]!, live: m[3] === 'live', target: Boolean(m[4]) }] : []
        })
    }

    async waitSessions(check: (rows: SessionRow[]) => boolean, timeoutMs = 5000): Promise<SessionRow[]> {
        return poll(async () => {
            const rows = await this.sessions()
            return check(rows) ? rows : undefined
        }, timeoutMs, 'the expected sessions')
    }

    async quit(): Promise<void> {
        const exited = new Promise(resolve => this.proc.once('exit', resolve))
        this.run('quit')
        await exited
    }
}

export class Channel {
    readonly messages: Record<string, any>[] = []
    readonly proc: ChildProcessWithoutNullStreams
    readonly sessionId: string
    #nextId = 1

    constructor(sessionId: string, cwd: string, discoveryPath: string) {
        this.sessionId = sessionId
        this.proc = spawn('node', [CHANNEL], {
            env: { ...process.env, CLAUDE_CODE_SESSION_ID: sessionId, CLAUDE_PROJECT_DIR: cwd, VOICE_BRIDGE_DISCOVERY: discoveryPath },
        })
        createInterface({ input: this.proc.stdout }).on('line', line => this.messages.push(JSON.parse(line)))
    }

    #write(message: object): void {
        this.proc.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', ...message })}\n`)
    }

    async request(method: string, params: object): Promise<Record<string, any>> {
        const id = this.#nextId++
        this.#write({ id, method, params })
        return poll(() => this.messages.find(m => m.id === id), 5000, `${method} response`)
    }

    async initialize(): Promise<void> {
        await this.request('initialize', {
            protocolVersion: '2025-06-18',
            capabilities: {},
            clientInfo: { name: 'e2e', version: '0' },
        })
        this.#write({ method: 'notifications/initialized' })
    }

    received(): string[] {
        return this.messages.filter(m => m.method === 'notifications/claude/channel').map(m => m.params.content)
    }

    async waitReceived(count: number): Promise<string[]> {
        return poll(() => (this.received().length >= count ? this.received() : undefined), 5000, `${count} channel messages`)
    }

    async reply(text: string): Promise<Record<string, any>> {
        return this.request('tools/call', { name: 'reply', arguments: { text } })
    }

    async kill(): Promise<void> {
        const exited = new Promise(resolve => this.proc.once('exit', resolve))
        this.proc.kill()
        await exited
    }
}
