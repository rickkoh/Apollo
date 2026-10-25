import assert from 'node:assert/strict'
import { test } from 'node:test'
import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js'
import type { HubClientEvents } from '../../src/hub-client.ts'
import { INSTRUCTIONS, NOT_RUNNING, createChannelServer, forwardDeliveries } from '../../src/mcp.ts'

async function connect(hub: { connected: boolean; reply: (text: string) => Promise<void> }) {
    const server = createChannelServer(hub)
    const client = new Client({ name: 'test', version: '0' })
    const [a, b] = InMemoryTransport.createLinkedPair()
    await Promise.all([server.connect(a), client.connect(b)])
    return client
}

function text(result: Awaited<ReturnType<Client['callTool']>>): string {
    return (result.content as { text: string }[])[0]!.text
}

test('declares the channel capability, the instructions and the reply tool', async () => {
    const client = await connect({ connected: true, reply: async () => {} })
    assert.deepEqual(client.getServerCapabilities()?.experimental, { 'claude/channel': {} })
    const instructions = client.getInstructions() ?? ''
    assert.equal(instructions, INSTRUCTIONS)
    assert.ok(instructions.startsWith('The user cannot see this terminal'))
    assert.ok(instructions.includes('plain text never reaches them'))
    const { tools } = await client.listTools()
    assert.deepEqual(tools.map(t => t.name), ['reply'])
})

test('reply forwards to the hub', async () => {
    const sent: string[] = []
    const client = await connect({ connected: true, reply: async t => void sent.push(t) })
    const result = await client.callTool({ name: 'reply', arguments: { text: 'Clicked it.' } })
    assert.equal(result.isError, false)
    assert.deepEqual(sent, ['Clicked it.'])
})

test('reply while disconnected tells Claude the user did not see it', async () => {
    const client = await connect({ connected: false, reply: async () => {} })
    const result = await client.callTool({ name: 'reply', arguments: { text: 'hi' } })
    assert.equal(result.isError, true)
    assert.equal(text(result), NOT_RUNNING)
})

test('the delivery is acknowledged only after the notification is written', async () => {
    const order: string[] = []
    let deliver: HubClientEvents['deliver'] = () => {}
    let release = () => {}
    forwardDeliveries(
        {
            on: ((event: string, listener: HubClientEvents['deliver']) => {
                if (event === 'deliver') deliver = listener
            }) as never,
            delivered: id => order.push(`delivered ${id}`),
        },
        text => {
            order.push(`notify ${text}`)
            return new Promise(resolve => (release = resolve))
        },
    )
    deliver({ type: 'deliver', id: 7, text: 'go', at: '' })
    await new Promise(resolve => setTimeout(resolve, 10))
    assert.deepEqual(order, ['notify go'])
    release()
    await new Promise(resolve => setTimeout(resolve, 10))
    assert.deepEqual(order, ['notify go', 'delivered 7'])
})
