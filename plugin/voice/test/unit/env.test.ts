import assert from 'node:assert/strict'
import { test } from 'node:test'
import { hostOf, readEnv } from '../../src/env.ts'

const base = { CLAUDE_CODE_SESSION_ID: 'abc', CLAUDE_PROJECT_DIR: '/w/repo', HOME: '/Users/me' }

test('reads the session and defaults the discovery path to the app data folder', () => {
    const env = readEnv(base, 77, new Date('2026-10-25T12:00:00Z'))
    assert.deepEqual(env, {
        sessionId: 'abc',
        projectDir: '/w/repo',
        claudePid: 77,
        startedAt: '2026-10-25T12:00:00.000Z',
        discoveryPath: '/Users/me/Library/Application Support/com.rickkoh.apollo/hub.json',
    })
})

test('APOLLO_DISCOVERY overrides the discovery path', () => {
    const env = readEnv({ ...base, APOLLO_DISCOVERY: '/tmp/vb/hub.json' }, 1)
    assert.ok(!('error' in env))
    assert.equal(env.discoveryPath, '/tmp/vb/hub.json')
})

test('a missing session id is an error', () => {
    const env = readEnv({ CLAUDE_PROJECT_DIR: '/w', HOME: '/h' }, 1)
    assert.ok('error' in env && env.error.includes('CLAUDE_CODE_SESSION_ID'))
})

test('falls back to PWD when CLAUDE_PROJECT_DIR is missing', () => {
    const env = readEnv({ CLAUDE_CODE_SESSION_ID: 'a', PWD: '/p', HOME: '/h' }, 1)
    assert.ok(!('error' in env))
    assert.equal(env.projectDir, '/p')
})

test('the host is a terminal unless the panel bridge says otherwise', () => {
    assert.equal(hostOf(base), 'terminal')
    assert.equal(hostOf({ ...base, APOLLO_HOST: 'panel' }), 'panel')
    assert.equal(hostOf({ ...base, APOLLO_HOST: 'other' }), 'terminal')
})
