import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Message } from '../shared/types'
import { FADE_MS, MAX_CARDS, MessageStack } from './stack.ts'

let next = 0
function message(direction: Message['direction'], text: string): Message {
    next += 1
    return {
        id: next,
        session: 's',
        direction,
        text,
        at: '2026-10-25T12:00:00Z',
        status: direction === 'to_session' ? 'queued' : 'received',
        detail: null,
    }
}

const texts = (s: MessageStack) => s.cards.map(c => `${c.kind}:${c.text}`)

test('sends stack up while nothing has come back, and stay', () => {
    const s = new MessageStack()
    s.sent(message('to_session', 'one'))
    s.sent(message('to_session', 'two'))
    s.tick(FADE_MS * 2, false)
    assert.deepEqual(texts(s), ['sent:one', 'sent:two'])
    assert.ok(s.pending)
})

test('a reply starts the fade of the echoes before it; later echoes keep waiting', () => {
    const s = new MessageStack()
    s.sent(message('to_session', 'one'))
    s.reply(message('from_session', 'r1'))
    s.sent(message('to_session', 'two'))
    s.tick(FADE_MS - 1, false)
    assert.equal(s.cards.length, 3)
    s.tick(1, false)
    assert.deepEqual(texts(s), ['sent:two'])
})

test('at most three cards, oldest dropped first', () => {
    const s = new MessageStack()
    for (const t of ['a', 'b', 'c', 'd']) s.reply(message('from_session', t))
    assert.equal(s.cards.length, MAX_CARDS)
    assert.deepEqual(texts(s), ['reply:b', 'reply:c', 'reply:d'])
})

test('hovering pauses every fade and an expanded card keeps its own', () => {
    const s = new MessageStack()
    s.reply(message('from_session', 'r1'))
    s.reply(message('from_session', 'r2'))
    s.tick(FADE_MS, true)
    assert.equal(s.cards.length, 2)
    s.cards[0]!.expanded = true
    s.tick(FADE_MS, false)
    assert.deepEqual(texts(s), ['reply:r1'])
})

test('a failed delivery becomes an error that stays until the next send, and returns the text', () => {
    const s = new MessageStack()
    const m = message('to_session', 'lost')
    s.sent(m)
    assert.equal(s.updated(m.id, 'failed', 'no acknowledgement'), 'lost')
    s.tick(FADE_MS * 2, false)
    assert.deepEqual(texts(s), ['error:Not delivered: no acknowledgement'])
    s.beginSend()
    assert.equal(s.cards.length, 0)
})

test('the send result arriving after its delivered event does not regress the status', () => {
    const s = new MessageStack()
    const m = message('to_session', 'hi')
    s.sent(m)
    s.updated(m.id, 'delivered', null)
    s.sent(m)
    assert.equal(s.cards[0]!.message?.status, 'delivered')
    assert.ok(!s.pending)
})
