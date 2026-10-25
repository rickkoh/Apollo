import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { PassThrough } from 'node:stream'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import { PLUGIN_SETTINGS, bridge, eachLine } from '../../src/bridge.ts'

function setup() {
    const host = { input: new PassThrough(), output: new PassThrough() }
    const claude = { stdin: new PassThrough(), stdout: new PassThrough() }
    const toHost: string[] = []
    const toClaude: string[] = []
    const logs: string[] = []
    eachLine(host.output, line => toHost.push(line))
    eachLine(claude.stdin, line => toClaude.push(line))
    bridge(host, claude, { retryMs: 5, maxAttempts: 3, log: m => logs.push(m) })
    return { host, claude, toHost, toClaude, logs }
}

const settle = (ms = 20) => new Promise(resolve => setTimeout(resolve, ms))
const line = (value: object) => JSON.stringify(value) + '\n'
const initialize = line({ type: 'control_request', request_id: 'init-1', request: { subtype: 'initialize' } })
const answer = (id: string, extra: object = { subtype: 'success' }) =>
    line({ type: 'control_response', response: { request_id: id, ...extra } })
const notConnected = { subtype: 'error', error: 'server plugin:voice:voice is not connected' }
const enables = (lines: string[]) => lines.filter(l => l.includes('"subtype":"channel_enable"'))

test('relays lines both ways unchanged, across chunk boundaries', async () => {
    const b = setup()
    b.host.input.write('{"type":"user","message":')
    b.host.input.write('"hi"}\n{"type":"keep_alive"}\n')
    b.claude.stdout.write('{"type":"assistant"}\n{"type":"res')
    b.claude.stdout.write('ult"}\n')
    await settle()
    assert.deepEqual(b.toClaude, ['{"type":"user","message":"hi"}', '{"type":"keep_alive"}'])
    assert.deepEqual(b.toHost, ['{"type":"assistant"}', '{"type":"result"}'])
})

test('sends nothing of its own before claude answers the host\'s initialize', async () => {
    const b = setup()
    b.host.input.write(initialize)
    await settle()
    assert.deepEqual(enables(b.toClaude), [])
})

test('enables the channel after the initialize answer, which still reaches the host, and keeps its own answer', async () => {
    const b = setup()
    b.host.input.write(initialize)
    b.claude.stdout.write(answer('init-1'))
    await settle()
    assert.equal(b.toHost.length, 1)
    assert.ok(b.toHost[0]!.includes('init-1'))
    const [request] = enables(b.toClaude)
    assert.deepEqual(JSON.parse(request!), {
        type: 'control_request',
        request_id: 'voice-bridge-1',
        request: { subtype: 'channel_enable', serverName: 'plugin:voice:voice' },
    })
    b.claude.stdout.write(answer('voice-bridge-1'))
    await settle()
    assert.equal(b.toHost.length, 1, 'our answer never reaches the host')
    assert.deepEqual(b.logs, ['voice channel enabled'])
})

test('retries while the server is still connecting and stops once enabled', async () => {
    const b = setup()
    b.host.input.write(initialize)
    b.claude.stdout.write(answer('init-1'))
    await settle()
    b.claude.stdout.write(answer('voice-bridge-1', notConnected))
    await settle()
    b.claude.stdout.write(answer('voice-bridge-2'))
    await settle()
    assert.equal(enables(b.toClaude).length, 2)
    assert.equal(b.toHost.length, 1)
})

test('gives up on a refusal, and after the last attempt', async () => {
    const refused = setup()
    refused.host.input.write(initialize)
    refused.claude.stdout.write(answer('init-1'))
    await settle()
    refused.claude.stdout.write(answer('voice-bridge-1', { subtype: 'error', error: 'not on the approved channels allowlist' }))
    await settle()
    assert.equal(enables(refused.toClaude).length, 1)
    assert.match(refused.logs[0]!, /allowlist/)

    const late = setup()
    late.host.input.write(initialize)
    late.claude.stdout.write(answer('init-1'))
    for (let n = 1; n <= 3; n++) {
        await settle()
        late.claude.stdout.write(answer(`voice-bridge-${n}`, notConnected))
    }
    await settle()
    assert.equal(enables(late.toClaude).length, 3)
    assert.match(late.logs[0]!, /not connected/)
})

test('ends claude\'s input when the host\'s ends', async () => {
    const b = setup()
    let ended = false
    b.claude.stdin.on('finish', () => (ended = true))
    b.host.input.end()
    await settle()
    assert.ok(ended)
})

test('the entry point adds the plugin settings and the panel marker, and exits with claude\'s code', () => {
    const main = fileURLToPath(new URL('../../src/bridge-main.ts', import.meta.url))
    const fake = fileURLToPath(new URL('./fake-claude.mjs', import.meta.url))
    const run = spawnSync(process.execPath, [main, process.execPath, fake, '--input-format', 'stream-json'], {
        input: '',
        encoding: 'utf8',
    })
    assert.equal(run.status, 3)
    assert.deepEqual(JSON.parse(run.stdout), {
        args: ['--input-format', 'stream-json', '--settings', PLUGIN_SETTINGS],
        host: 'panel',
    })
})
