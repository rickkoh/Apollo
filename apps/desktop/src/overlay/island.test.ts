import assert from 'node:assert/strict'
import { test } from 'node:test'
import { DWELL_MS, GRACE_MS, Island } from './island.ts'

const PEEK_MS = 20_000

function at(island: Island, now: number): Island {
    island.advance(now)
    return island
}

test('resting on the island opens it; a quick pass does not', () => {
    const island = new Island()
    island.pointer(true, 0)
    island.pointer(false, DWELL_MS - 50)
    assert.equal(at(island, DWELL_MS).state, 'hidden')
    island.pointer(true, 1000)
    assert.equal(at(island, 1000 + DWELL_MS - 1).state, 'hidden')
    assert.equal(at(island, 1000 + DWELL_MS).state, 'open')
    assert.equal(island.due, null)
})

test('leaving closes it after the grace; coming back within it keeps it open', () => {
    const island = new Island()
    island.pointer(true, 0)
    island.advance(DWELL_MS)
    island.pointer(false, 500)
    island.pointer(true, 500 + GRACE_MS - 10)
    assert.equal(at(island, 2000).state, 'open')
    island.pointer(false, 3000)
    assert.equal(at(island, 3000 + GRACE_MS - 1).state, 'open')
    assert.equal(at(island, 3000 + GRACE_MS).state, 'hidden')
})

test('Esc closes it until the pointer leaves and comes back', () => {
    const island = new Island()
    island.pointer(true, 0)
    island.advance(DWELL_MS)
    island.escape()
    assert.equal(at(island, 5000).state, 'hidden')
    island.pointer(false, 6000)
    island.pointer(true, 7000)
    assert.equal(at(island, 7000 + DWELL_MS).state, 'open')
})

test('gaining the keyboard opens it; losing it closes it until the pointer comes back', () => {
    const island = new Island()
    island.pointer(true, 0)
    island.keyboard(true, 10)
    assert.equal(island.state, 'open')
    island.keyboard(false, 500)
    assert.equal(at(island, 5000).state, 'hidden')
    island.pointer(false, 6000)
    island.pointer(true, 6100)
    assert.equal(at(island, 6100 + DWELL_MS).state, 'open')
})

test('opened by the keyboard with the pointer elsewhere, it closes after the grace', () => {
    const island = new Island()
    island.keyboard(true, 0)
    assert.equal(island.state, 'open')
    assert.equal(at(island, GRACE_MS).state, 'hidden')
})

test('a reply peeks, then closes to compact until the island opens', () => {
    const island = new Island()
    island.reply(0, PEEK_MS)
    assert.equal(island.state, 'peek')
    assert.equal(at(island, PEEK_MS - 1).state, 'peek')
    assert.equal(at(island, PEEK_MS).state, 'compact')
    island.pointer(true, PEEK_MS + 100)
    island.advance(PEEK_MS + 100 + DWELL_MS)
    assert.equal(island.state, 'open')
    assert.equal(island.unseen, false)
    island.pointer(false, 30_000)
    assert.equal(at(island, 30_000 + GRACE_MS).state, 'hidden')
})

test('hovering a peek opens it', () => {
    const island = new Island()
    island.reply(0, PEEK_MS)
    island.pointer(true, 1000)
    assert.equal(at(island, 1000 + DWELL_MS).state, 'open')
    island.pointer(false, 2000)
    assert.equal(at(island, PEEK_MS + 1).state, 'hidden')
})

test('an error peeks without counting as unseen', () => {
    const island = new Island()
    island.error(0, PEEK_MS)
    assert.equal(island.state, 'peek')
    assert.equal(island.unseen, false)
    assert.equal(at(island, PEEK_MS).state, 'hidden')
})

test('a reply while open changes nothing', () => {
    const island = new Island()
    island.pointer(true, 0)
    island.advance(DWELL_MS)
    island.reply(1000, PEEK_MS)
    assert.equal(island.state, 'open')
    assert.equal(island.unseen, false)
})

test('news shows the wings while closed', () => {
    const island = new Island()
    island.news = true
    assert.equal(island.state, 'compact')
    island.news = false
    assert.equal(island.state, 'hidden')
})
