import { writeFileSync } from 'node:fs'
import { Server } from '@modelcontextprotocol/sdk/server/index.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js'

const HUB = 'http://127.0.0.1:8795'
const SESSION = process.env.CLAUDE_CODE_SESSION_ID
const CWD = process.env.CLAUDE_PROJECT_DIR ?? process.cwd()

const mcp = new Server(
    { name: 'voice', version: '0.0.1' },
    {
        capabilities: { experimental: { 'claude/channel': {} }, tools: {} },
        instructions:
            'The user cannot see this terminal: anything they should see must go through this server\'s reply tool, and plain text never reaches them. ' +
            'Messages from this voice channel are the user speaking to you, dictated while they watch a browser live view, so expect transcription slips. ' +
            'Answer every one, small talk included, by calling reply with one or two sentences on what you did or why you could not.',
    },
)

mcp.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [{
        name: 'reply',
        description: 'Show a short reply to the user in the voice overlay.',
        inputSchema: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] },
    }],
}))

mcp.setRequestHandler(CallToolRequestSchema, async req => {
    if (req.params.name !== 'reply') throw new Error(`unknown tool: ${req.params.name}`)
    await fetch(`${HUB}/reply?session=${encodeURIComponent(SESSION)}`, { method: 'POST', body: String(req.params.arguments?.text ?? '') })
    return { content: [{ type: 'text', text: 'sent' }] }
})

mcp.oninitialized = () => writeFileSync(
    new URL(`../clients/${SESSION}.json`, import.meta.url),
    JSON.stringify({ session: SESSION, cwd: CWD, client: mcp.getClientVersion(), capabilities: mcp.getClientCapabilities(), argv: process.argv }, null, 2),
)

await mcp.connect(new StdioServerTransport())

async function listen() {
    const url = `${HUB}/inbox?session=${encodeURIComponent(SESSION)}&cwd=${encodeURIComponent(CWD)}`
    for (;;) {
        try {
            const res = await fetch(url)
            const decoder = new TextDecoder()
            let buffer = ''
            for await (const chunk of res.body) {
                buffer += decoder.decode(chunk, { stream: true })
                let end
                while ((end = buffer.indexOf('\n\n')) >= 0) {
                    const event = buffer.slice(0, end)
                    buffer = buffer.slice(end + 2)
                    if (!event.startsWith('data: ')) continue
                    await mcp.notification({ method: 'notifications/claude/channel', params: { content: JSON.parse(event.slice(6)).text } })
                }
            }
        } catch (err) {
            console.error(`voice: hub unreachable (${err.message})`)
        }
        await new Promise(resolve => setTimeout(resolve, 2000))
    }
}

listen()
