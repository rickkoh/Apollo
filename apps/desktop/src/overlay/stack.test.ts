import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Message } from '../shared/types'
import { MessageStack } from './stack.ts'

const FADE_MS = 20_000
const MAX_CARDS = 3
const stack = () => new MessageStack({ fadeMs: FADE_MS, maxCards: MAX_CARDS })

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
    const s = stack()
    s.sent(message('to_session', 'one'))
    s.sent(message('to_session', 'two'))
    s.tick(FADE_MS * 2, false)
    assert.deepEqual(texts(s), ['sent:one', 'sent:two'])
    assert.ok(s.pending)
})

test('a reply starts the fade of the echoes before it; later echoes keep waiting', () => {
    const s = stack()
    s.sent(message('to_session', 'one'))
    s.reply(message('from_session', 'r1'))
    s.sent(message('to_session', 'two'))
    s.tick(FADE_MS - 1, false)
    assert.equal(s.cards.length, 3)
    s.tick(1, false)
    assert.deepEqual(texts(s), ['sent:two'])
})

test('at most three cards, oldest dropped first', () => {
    const s = stack()
    for (const t of ['a', 'b', 'c', 'd']) s.reply(message('from_session', t))
    assert.equal(s.cards.length, MAX_CARDS)
    assert.deepEqual(texts(s), ['reply:b', 'reply:c', 'reply:d'])
})

test('hovering pauses every fade and an expanded card keeps its own', () => {
    const s = stack()
    s.reply(message('from_session', 'r1'))
    s.reply(message('from_session', 'r2'))
    s.tick(FADE_MS, true)
    assert.equal(s.cards.length, 2)
    s.cards[0]!.expanded = true
    s.tick(FADE_MS, false)
    assert.deepEqual(texts(s), ['reply:r1'])
})

test('a failed delivery becomes an error that stays until the next send, and returns the text', () => {
    const s = stack()
    const m = message('to_session', 'lost')
    s.sent(m)
    assert.equal(s.updated(m.id, 'failed', 'no acknowledgement'), 'lost')
    s.tick(FADE_MS * 2, false)
    assert.deepEqual(texts(s), ['error:Not delivered: no acknowledgement'])
    s.beginSend()
    assert.equal(s.cards.length, 0)
})

test('the send result arriving after its delivered event does not regress the status', () => {
    const s = stack()
    const m = message('to_session', 'hi')
    s.sent(m)
    s.updated(m.id, 'delivered', null)
    s.sent(m)
    assert.equal(s.cards[0]!.message?.status, 'delivered')
    assert.ok(!s.pending)
})

test('a note on a delivered message stays on its echo, which keeps waiting', () => {
    const s = stack()
    const m = message('to_session', 'one')
    s.sent(m)
    assert.equal(s.updated(m.id, 'delivered', "Claude hasn't picked this up"), null)
    s.tick(FADE_MS * 2, false)
    assert.deepEqual(texts(s), ['sent:one'])
    assert.equal(s.cards[0]!.message?.detail, "Claude hasn't picked this up")
})

test('with no fade, cards stay until newer ones push them out', () => {
    const s = new MessageStack({ fadeMs: null, maxCards: MAX_CARDS })
    s.sent(message('to_session', 'one'))
    s.reply(message('from_session', 'r1'))
    s.tick(FADE_MS * 10, false)
    assert.deepEqual(texts(s), ['sent:one', 'reply:r1'])
    for (const t of ['r2', 'r3']) s.reply(message('from_session', t))
    assert.deepEqual(texts(s), ['reply:r1', 'reply:r2', 'reply:r3'])
})

test('a smaller cap drops the oldest cards at once', () => {
    const s = stack()
    for (const t of ['a', 'b', 'c']) s.reply(message('from_session', t))
    s.configure({ fadeMs: FADE_MS, maxCards: 1 })
    assert.deepEqual(texts(s), ['reply:c'])
})

test('a new fade applies to cards that start fading afterwards', () => {
    const s = stack()
    s.reply(message('from_session', 'old'))
    s.configure({ fadeMs: FADE_MS * 3, maxCards: MAX_CARDS })
    s.reply(message('from_session', 'new'))
    s.tick(FADE_MS, false)
    assert.deepEqual(texts(s), ['reply:new'])
})
