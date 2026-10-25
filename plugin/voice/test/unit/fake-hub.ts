import type { AddressInfo } from 'node:net'
import { WebSocketServer, type WebSocket } from 'ws'
import type { Discovery } from '../../../../shared/types/Discovery'

export const TOKEN = 'b'.repeat(64)

/** A hub stand-in that records frames and lets a test push frames or drop connections. */
export class FakeHub {
    readonly frames: Record<string, unknown>[] = []
    readonly server: WebSocketServer
    socket: WebSocket | null = null
    connections = 0

    private constructor(server: WebSocketServer) {
        this.server = server
        server.on('connection', socket => {
            this.connections += 1
            this.socket = socket
            socket.on('message', data => {
                const frame = JSON.parse(String(data)) as Record<string, unknown>
                this.frames.push(frame)
                if (frame.type === 'hello') socket.send(JSON.stringify({ type: 'welcome', title: 'repo' }))
            })
        })
    }

    static async start(): Promise<FakeHub> {
        const server = new WebSocketServer({ host: '127.0.0.1', port: 0, path: '/channel' })
        await new Promise(resolve => server.once('listening', resolve))
        return new FakeHub(server)
    }

    get discovery(): Discovery {
        return { version: 1, port: (this.server.address() as AddressInfo).port, token: TOKEN, pid: 1 }
    }

    push(frame: object): void {
        this.socket?.send(JSON.stringify(frame))
    }

    async close(): Promise<void> {
        for (const client of this.server.clients) client.terminate()
        await new Promise(resolve => this.server.close(resolve))
    }
}

export async function until(check: () => boolean, timeoutMs = 2000): Promise<void> {
    const end = Date.now() + timeoutMs
    while (!check()) {
        if (Date.now() > end) throw new Error('condition not met in time')
        await new Promise(resolve => setTimeout(resolve, 10))
    }
}
