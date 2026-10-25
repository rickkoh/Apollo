import assert from 'node:assert/strict'
import { mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { readDiscovery } from '../../src/discovery.ts'

const token = 'a'.repeat(64)

test('missing, invalid and valid files', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'vb-'))
    const path = join(dir, 'hub.json')
    assert.equal(await readDiscovery(path), null)
    await writeFile(path, '{not json')
    assert.equal(await readDiscovery(path), null)
    await writeFile(path, JSON.stringify({ version: 2, port: 1, token, pid: 1 }))
    assert.equal(await readDiscovery(path), null)
    await writeFile(path, JSON.stringify({ version: 1, port: 51234, token, pid: 42 }))
    assert.deepEqual(await readDiscovery(path), { version: 1, port: 51234, token, pid: 42 })
})
