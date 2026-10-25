import { spawn } from 'node:child_process'
import { constants } from 'node:os'
import { PLUGIN_SETTINGS, bridge } from './bridge.ts'

// cv-panel runs this as `panel-bridge.js <claude binary> <args...>` for IDE panel sessions only.
const log = (message: string) => process.stderr.write(`apollo: ${message}\n`)
const [binary, ...args] = process.argv.slice(2)
if (!binary) {
    log('usage: panel-bridge.js <claude binary> <args...>')
    process.exit(2)
}

const claude = spawn(binary, [...args, '--settings', PLUGIN_SETTINGS], {
    env: { ...process.env, APOLLO_HOST: 'panel' },
    stdio: ['pipe', 'pipe', 'inherit'],
})
bridge({ input: process.stdin, output: process.stdout }, claude, { log })

for (const signal of ['SIGTERM', 'SIGINT', 'SIGHUP'] as const) process.on(signal, () => claude.kill(signal))
claude.on('error', err => {
    log(`cannot start ${binary}: ${err.message}`)
    process.exit(1)
})
claude.on('close', (code, signal) => {
    const status = code ?? (signal ? 128 + constants.signals[signal] : 1)
    process.stdout.end(() => process.exit(status))
})
