import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Discovery } from '../../../../shared/types/Discovery'
import type { ChannelEnv } from '../../src/env.ts'
import { HubClient, type Deliver } from '../../src/hub-client.ts'
import { FakeHub, TOKEN, until } from './fake-hub.ts'

const env: ChannelEnv = {
    sessionId: 's1',
    projectDir: '/w/repo',
    claudePid: 9,
    startedAt: '2026-10-25T12:00:00.000Z',
    discoveryPath: 'unused',
}
const fast = { backoff: [20, 20, 50] }

test('hello carries the token and session; welcome connects; deliver is emitted', async () => {
    const hub = await FakeHub.start()
    const client = new HubClient(env, async () => hub.discovery, fast)
    const titles: string[] = []
    const delivered: Deliver[] = []
    client.on('connected', t => titles.push(t)).on('deliver', f => delivered.push(f))
    client.start()
    await until(() => client.connected)
    assert.deepEqual(hub.frames[0], {
        type: 'hello',
        protocol: 1,
        token: TOKEN,
        session: { id: 's1', cwd: '/w/repo', claudePid: 9, startedAt: '2026-10-25T12:00:00.000Z' },
    })
    assert.deepEqual(titles, ['repo'])
    hub.push({ type: 'deliver', id: 4, text: 'hi', at: '2026-10-25T12:00:01Z' })
    await until(() => delivered.length === 1)
    client.delivered(4)
    await client.reply('done')
    await until(() => hub.frames.length === 3)
    assert.deepEqual(hub.frames.slice(1), [{ type: 'delivered', id: 4 }, { type: 'reply', text: 'done' }])
    client.stop()
    await hub.close()
})

test('retries while there is no discovery file, then connects', async () => {
    const hub = await FakeHub.start()
    let reads = 0
    const client = new HubClient(env, async () => (++reads < 3 ? null : hub.discovery), fast)
    client.start()
    await until(() => client.connected)
    assert.equal(reads, 3)
    client.stop()
    await hub.close()
})

test('reconnects to a new port after the hub restarts, re-reading discovery', async () => {
    let hub = await FakeHub.start()
    const client = new HubClient(env, async () => hub.discovery, fast)
    let disconnects = 0
    client.on('disconnected', () => disconnects++)
    client.start()
    await until(() => client.connected)
    const oldPort = hub.discovery.port
    await hub.close()
    await until(() => !client.connected)
    hub = await FakeHub.start()
    assert.notEqual(hub.discovery.port, oldPort)
    await until(() => client.connected)
    assert.equal(disconnects, 1)
    client.stop()
    await hub.close()
})

test('reply rejects while disconnected', async () => {
    const client = new HubClient(env, async () => null, fast)
    await assert.rejects(client.reply('x'), /not connected/)
})

test('a replaced error stops the client for good', async () => {
    const hub = await FakeHub.start()
    const client = new HubClient(env, async () => hub.discovery, fast)
    client.start()
    await until(() => client.connected)
    hub.push({ type: 'error', code: 'replaced', message: 'another connection took over' })
    await until(() => !client.connected)
    await new Promise(resolve => setTimeout(resolve, 150))
    assert.equal(hub.connections, 1)
    await hub.close()
})

test('other errors retry with backoff', async () => {
    const hub = await FakeHub.start()
    const client = new HubClient(env, async () => hub.discovery, fast)
    client.start()
    await until(() => client.connected)
    hub.push({ type: 'error', code: 'bad_token', message: 'token mismatch' })
    hub.socket?.close()
    await until(() => hub.connections === 2 && client.connected)
    client.stop()
    await hub.close()
})
