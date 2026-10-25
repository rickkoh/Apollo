import http from 'node:http'

const PORT = 8795
const HOSTS = new Set([`127.0.0.1:${PORT}`, `localhost:${PORT}`])
const ORIGINS = new Set([...HOSTS].map(host => `http://${host}`))

const sessions = new Map()
const watchers = new Set()

function allowed(req) {
    const origin = req.headers.origin
    return HOSTS.has(req.headers.host) && (!origin || ORIGINS.has(origin))
}

function stream(res) {
    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache' })
    res.flushHeaders()
}

function emit(res, event) {
    res.write(`data: ${JSON.stringify(event)}\n\n`)
}

async function body(req) {
    let text = ''
    for await (const chunk of req) text += chunk
    return text.trim()
}

http.createServer(async (req, res) => {
    if (!allowed(req)) return res.writeHead(403).end()
    const url = new URL(req.url, `http://${req.headers.host}`)
    const id = url.searchParams.get('session')

    if (req.method === 'GET' && url.pathname === '/inbox' && id) {
        stream(res)
        const entry = { cwd: url.searchParams.get('cwd'), since: new Date().toISOString(), res }
        sessions.set(id, entry)
        console.log(`registered ${id} ${entry.cwd}`)
        req.on('close', () => {
            if (sessions.get(id) === entry) sessions.delete(id)
            console.log(`gone ${id}`)
        })
        return
    }

    if (req.method === 'GET' && url.pathname === '/sessions') {
        const list = [...sessions].map(([id, { cwd, since }]) => ({ id, cwd, since }))
        return res.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify(list, null, 2))
    }

    if (req.method === 'POST' && url.pathname === '/send' && id) {
        const session = sessions.get(id)
        if (!session) return res.writeHead(404).end('no such session')
        emit(session.res, { text: await body(req) })
        return res.writeHead(204).end()
    }

    if (req.method === 'POST' && url.pathname === '/reply' && id) {
        const event = { session: id, text: await body(req) }
        console.log(`reply ${id}: ${event.text}`)
        for (const watcher of watchers) emit(watcher, event)
        return res.writeHead(204).end()
    }

    if (req.method === 'GET' && url.pathname === '/events') {
        stream(res)
        watchers.add(res)
        req.on('close', () => watchers.delete(res))
        return
    }

    res.writeHead(404).end()
}).listen(PORT, '127.0.0.1', () => console.log(`hub on ${PORT}`))
