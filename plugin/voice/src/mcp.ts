import { Server } from '@modelcontextprotocol/sdk/server/index.js'
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js'
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js'
import pkg from '../package.json' with { type: 'json' }
import type { Host } from './env.ts'
import type { HubClient } from './hub-client.ts'

export const INSTRUCTIONS =
    'The user cannot see this terminal: anything they should see must go through this server\'s reply tool, and plain ' +
    'text never reaches them. Messages from this voice channel are the user speaking to you, dictated while they ' +
    'watch the screen, so expect transcription slips. Answer every one, small talk included, by calling reply with one ' +
    'or two sentences on what you did or why you could not.'

export const PANEL_INSTRUCTIONS =
    'Messages from this voice channel are the user speaking to you from the Voice Bridge overlay, dictated while they ' +
    'may be looking away from this chat, so expect transcription slips. Answer every one, small talk included, by ' +
    'calling this server\'s reply tool with one or two sentences on what you did or why you could not. Answer messages ' +
    'typed in this chat in the chat, as usual.'

export const NOT_RUNNING = 'Voice Bridge is not running, so the user did not see this. Tell them in the terminal instead.'

const MAX_REPLY_CHARS = 16000

function result(text: string, isError = false): CallToolResult {
    return { content: [{ type: 'text', text }], isError }
}

export function createChannelServer(hub: Pick<HubClient, 'reply' | 'connected'>, host: Host = 'terminal'): Server {
    const server = new Server(
        { name: 'voice', version: pkg.version },
        {
            capabilities: { experimental: { 'claude/channel': {} }, tools: {} },
            instructions: host === 'panel' ? PANEL_INSTRUCTIONS : INSTRUCTIONS,
        },
    )

    server.setRequestHandler(ListToolsRequestSchema, async () => ({
        tools: [{
            name: 'reply',
            description: 'Show a short reply to the user in the Voice Bridge overlay.',
            inputSchema: {
                type: 'object',
                properties: { text: { type: 'string', maxLength: MAX_REPLY_CHARS } },
                required: ['text'],
            },
        }],
    }))

    server.setRequestHandler(CallToolRequestSchema, async request => {
        if (request.params.name !== 'reply') return result(`unknown tool: ${request.params.name}`, true)
        const text = request.params.arguments?.text
        if (typeof text !== 'string' || !text.trim()) return result('reply needs a non-empty text', true)
        if (text.length > MAX_REPLY_CHARS) return result(`reply text is limited to ${MAX_REPLY_CHARS} characters`, true)
        if (!hub.connected) return result(NOT_RUNNING, true)
        try {
            await hub.reply(text)
            return result('shown to the user')
        } catch {
            return result(NOT_RUNNING, true)
        }
    })

    return server
}

/** Writes each delivery into the session, then acknowledges it, so `delivered` means "in the session's transport". */
export function forwardDeliveries(hub: Pick<HubClient, 'on' | 'delivered'>, notify: (text: string) => Promise<void>): void {
    hub.on('deliver', frame => {
        notify(frame.text).then(() => hub.delivered(frame.id), () => {})
    })
}

export function channelNotifier(server: Server): (text: string) => Promise<void> {
    return text => server.notification({ method: 'notifications/claude/channel', params: { content: text } })
}
