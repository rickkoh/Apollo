import { join } from 'node:path'

export interface ChannelEnv {
    sessionId: string
    projectDir: string
    claudePid: number
    startedAt: string
    discoveryPath: string
}

/** Where the session runs: a terminal (`cv`) or an IDE panel, whose bridge sets `VOICE_BRIDGE_HOST=panel`. */
export type Host = 'terminal' | 'panel'

const APP_DATA = ['Library', 'Application Support', 'dev.voicebridge.desktop']

export function hostOf(env: NodeJS.ProcessEnv): Host {
    return env.VOICE_BRIDGE_HOST === 'panel' ? 'panel' : 'terminal'
}

export function readEnv(env: NodeJS.ProcessEnv, ppid: number, now = new Date()): ChannelEnv | { error: string } {
    const sessionId = env.CLAUDE_CODE_SESSION_ID
    if (!sessionId) return { error: 'CLAUDE_CODE_SESSION_ID is not set, so this channel cannot tell which session it serves' }
    const projectDir = env.CLAUDE_PROJECT_DIR ?? env.PWD
    if (!projectDir) return { error: 'neither CLAUDE_PROJECT_DIR nor PWD is set' }
    const home = env.HOME
    const discoveryPath = env.VOICE_BRIDGE_DISCOVERY ?? (home ? join(home, ...APP_DATA, 'hub.json') : undefined)
    if (!discoveryPath) return { error: 'HOME is not set, so the hub discovery file cannot be found' }
    return { sessionId, projectDir, claudePid: ppid, startedAt: now.toISOString(), discoveryPath }
}
