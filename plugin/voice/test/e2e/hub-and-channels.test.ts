import assert from 'node:assert/strict'
import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { after, before, test } from 'node:test'
import { Channel, HubDev } from './processes.ts'

let dir: string
let discovery: string
let hub: HubDev
const channels: Channel[] = []

function channel(id: string, folder: string): Channel {
    const c = new Channel(id, `/work/${folder}`, discovery)
    channels.push(c)
    return c
}

before(async () => {
    dir = await mkdtemp(join(tmpdir(), 'vb-e2e-'))
    discovery = join(dir, 'hub.json')
    hub = new HubDev(dir, discovery)
    await hub.ready()
})

after(async () => {
    for (const c of channels) if (c.proc.exitCode === null) c.proc.kill('SIGKILL')
    if (hub.proc.exitCode === null) await hub.quit()
})

test('each message reaches only its session, and replies are tagged with their session', async () => {
    const a = channel('aaaa-1111', 'alpha')
    const b = channel('bbbb-2222', 'beta')
    await Promise.all([a.initialize(), b.initialize()])
    await hub.waitSessions(rows => rows.filter(r => r.live).length === 2)

    hub.run('target aaaa')
    hub.run('send for alpha')
    assert.deepEqual(await a.waitReceived(1), ['for alpha'])

    hub.run('target bbbb')
    hub.run('send for beta')
    assert.deepEqual(await b.waitReceived(1), ['for beta'])
    await new Promise(resolve => setTimeout(resolve, 200))
    assert.deepEqual(a.received(), ['for alpha'])
    await hub.waitLine(/^message \d+ Delivered$/)

    const result = await b.reply('reply from beta')
    assert.equal(result.result.isError, false)
    assert.match(await hub.waitLine(/^reply /), /^reply \[beta bbbb-2222\] reply from beta$/)
})

test('a closed session drops to ended', async () => {
    const a = channels[0]!
    await a.kill()
    const rows = await hub.waitSessions(rows => rows.some(r => r.id === a.sessionId && !r.live))
    assert.ok(rows.find(r => r.id === 'bbbb-2222')?.live)
})

test('after a hub restart the channels reconnect within 10 s', async () => {
    await hub.quit()
    hub = new HubDev(dir, discovery)
    await hub.ready()
    const started = Date.now()
    await hub.waitSessions(rows => rows.some(r => r.id === 'bbbb-2222' && r.live), 10000)
    assert.ok(Date.now() - started < 10000)
})

test('a delivery nobody acknowledges fails after the timeout', async () => {
    const b = channels[1]!
    hub.run('target bbbb')
    b.proc.kill('SIGSTOP')
    try {
        hub.run('send nobody hears this')
        assert.match(await hub.waitLine(/^message \d+ Failed: no acknowledgement$/, 8000), /no acknowledgement/)
    } finally {
        b.proc.kill('SIGCONT')
    }
})
