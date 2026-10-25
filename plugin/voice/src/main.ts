import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { readDiscovery } from './discovery.ts'
import { readEnv } from './env.ts'
import { HubClient } from './hub-client.ts'
import { log } from './log.ts'
import { channelNotifier, createChannelServer, forwardDeliveries } from './mcp.ts'

const env = readEnv(process.env, process.ppid)

if ('error' in env) {
    log(`${env.error}; staying idle`)
    await createChannelServer({ connected: false, reply: async () => {} }).connect(new StdioServerTransport())
} else {
    const hub = new HubClient(env, () => readDiscovery(env.discoveryPath))
    const server = createChannelServer(hub)
    forwardDeliveries(hub, channelNotifier(server))
    await server.connect(new StdioServerTransport())
    hub.start()
    const stop = () => {
        hub.stop()
        process.exit(0)
    }
    process.stdin.on('close', stop)
    process.on('SIGTERM', stop)
}
