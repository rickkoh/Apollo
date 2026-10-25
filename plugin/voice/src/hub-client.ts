import type { ChannelFrame } from '../../../shared/types/ChannelFrame'
import type { HubFrame } from '../../../shared/types/HubFrame'
import type { Discovery } from './discovery.ts'
import type { ChannelEnv } from './env.ts'
import { log } from './log.ts'

export type Deliver = Extract<HubFrame, { type: 'deliver' }>

export interface HubClientEvents {
    deliver(frame: Deliver): void
    connected(title: string): void
    disconnected(): void
}

export interface HubClientOptions {
    /** Delays before each reconnect attempt; the last one repeats. */
    backoff?: readonly number[]
}

const PROTOCOL_VERSION = 1
const DEFAULT_BACKOFF = [1000, 2000, 5000, 10000] as const

type Listeners = { [K in keyof HubClientEvents]: HubClientEvents[K][] }

export class HubClient {
    readonly #env: ChannelEnv
    readonly #readDiscovery: () => Promise<Discovery | null>
    readonly #backoff: readonly number[]
    readonly #listeners: Listeners = { deliver: [], connected: [], disconnected: [] }
    #socket: WebSocket | null = null
    #welcomed = false
    #attempt = 0
    #stopped = true
    #timer: NodeJS.Timeout | null = null

    constructor(env: ChannelEnv, readDiscovery: () => Promise<Discovery | null>, options: HubClientOptions = {}) {
        this.#env = env
        this.#readDiscovery = readDiscovery
        this.#backoff = options.backoff ?? DEFAULT_BACKOFF
    }

    get connected(): boolean {
        return this.#welcomed
    }

    on<K extends keyof HubClientEvents>(event: K, listener: HubClientEvents[K]): this {
        this.#listeners[event].push(listener)
        return this
    }

    start(): void {
        if (!this.#stopped) return
        this.#stopped = false
        void this.#connect()
    }

    stop(): void {
        this.#stopped = true
        if (this.#timer) clearTimeout(this.#timer)
        this.#timer = null
        this.#socket?.close(1000)
        this.#socket = null
        this.#markDisconnected()
    }

    delivered(id: number): void {
        this.#send({ type: 'delivered', id })
    }

    async reply(text: string): Promise<void> {
        if (!this.#welcomed) throw new Error('not connected to Apollo')
        this.#send({ type: 'reply', text })
    }

    #emit<K extends keyof HubClientEvents>(event: K, ...args: Parameters<HubClientEvents[K]>): void {
        for (const listener of this.#listeners[event]) (listener as (...a: Parameters<HubClientEvents[K]>) => void)(...args)
    }

    #send(frame: ChannelFrame): void {
        if (this.#socket?.readyState === WebSocket.OPEN) this.#socket.send(JSON.stringify(frame))
    }

    #retry(): void {
        if (this.#stopped || this.#timer) return
        const delay = this.#backoff[Math.min(this.#attempt, this.#backoff.length - 1)] ?? 10000
        this.#attempt += 1
        this.#timer = setTimeout(() => {
            this.#timer = null
            void this.#connect()
        }, delay)
    }

    async #connect(): Promise<void> {
        if (this.#stopped) return
        const discovery = await this.#readDiscovery()
        if (this.#stopped) return
        if (!discovery) return this.#retry()

        const socket = new WebSocket(`ws://127.0.0.1:${discovery.port}/channel`)
        this.#socket = socket
        socket.addEventListener('open', () => {
            const { sessionId: id, projectDir: cwd, claudePid, startedAt } = this.#env
            this.#send({ type: 'hello', protocol: PROTOCOL_VERSION, token: discovery.token, session: { id, cwd, claudePid, startedAt } })
        })
        socket.addEventListener('message', event => this.#onFrame(String(event.data)))
        socket.addEventListener('close', () => {
            if (this.#socket !== socket) return
            this.#socket = null
            this.#markDisconnected()
            this.#retry()
        })
        socket.addEventListener('error', () => {})
    }

    #markDisconnected(): void {
        if (!this.#welcomed) return
        this.#welcomed = false
        log('disconnected from Apollo')
        this.#emit('disconnected')
    }

    #onFrame(data: string): void {
        let frame: HubFrame
        try {
            frame = JSON.parse(data) as HubFrame
        } catch {
            return log('ignoring a frame that is not JSON')
        }
        switch (frame.type) {
            case 'welcome':
                this.#attempt = 0
                this.#welcomed = true
                log(`connected to Apollo as "${frame.title}"`)
                this.#emit('connected', frame.title)
                break
            case 'deliver':
                this.#emit('deliver', frame)
                break
            case 'error':
                log(`hub refused: ${frame.code} (${frame.message})`)
                if (frame.code === 'replaced') this.stop()
                break
        }
    }
}
