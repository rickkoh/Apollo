import { readFileSync } from 'node:fs'
import http from 'node:http'
import { Server } from '@modelcontextprotocol/sdk/server/index.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js'

const PORT = Number(process.env.VOICE_BRIDGE_PORT ?? 8790)
const PAGE = new URL('./page.html', import.meta.url)
// Host and Origin checks keep other web pages (and DNS rebinding) from posting into the session.
const HOSTS = new Set([`127.0.0.1:${PORT}`, `localhost:${PORT}`])
const ORIGINS = new Set([...HOSTS].map(host => `http://${host}`))

const mcp = new Server(
    { name: 'voice', version: '0.0.1' },
    {
        capabilities: { experimental: { 'claude/channel': {} }, tools: {} },
        instructions:
            'The user cannot see this terminal: anything they should see must go through mcp__voice__reply, and plain text never reaches them. ' +
            'Messages in <channel source="voice"> are the user speaking to you, dictated while they watch a browser live view, so expect transcription slips. ' +
            'Answer every one, small talk included, by calling mcp__voice__reply with one or two sentences on what you did or why you could not.',
    },
)

mcp.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [{
        name: 'reply',
        description: 'Show a short reply to the user in the voice bridge window.',
        inputSchema: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] },
    }],
}))

mcp.setRequestHandler(CallToolRequestSchema, async req => {
    if (req.params.name !== 'reply') throw new Error(`unknown tool: ${req.params.name}`)
    broadcast({ text: String(req.params.arguments?.text ?? '') })
    return { content: [{ type: 'text', text: 'sent' }] }
})

await mcp.connect(new StdioServerTransport())

const listeners = new Set()

function broadcast(event) {
    const line = `data: ${JSON.stringify(event)}\n\n`
    for (const res of listeners) res.write(line)
}

function allowed(req) {
    const origin = req.headers.origin
    return HOSTS.has(req.headers.host) && (!origin || ORIGINS.has(origin))
}

http.createServer(async (req, res) => {
    if (!allowed(req)) return res.writeHead(403).end()

    if (req.method === 'GET' && req.url === '/') {
        return res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }).end(readFileSync(PAGE))
    }

    if (req.method === 'GET' && req.url === '/events') {
        res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache' })
        res.flushHeaders()
        listeners.add(res)
        req.on('close', () => listeners.delete(res))
        return
    }

    if (req.method === 'POST' && req.url === '/send') {
        let body = ''
        for await (const chunk of req) body += chunk
        const text = body.trim()
        if (text) await mcp.notification({ method: 'notifications/claude/channel', params: { content: text } })
        return res.writeHead(204).end()
    }

    res.writeHead(404).end()
})
    .on('error', err => console.error(`voice bridge: ${err.message}`))
    .listen(PORT, '127.0.0.1')
