/** stdout is the MCP transport, so every log line goes to stderr. */
export function log(message: string): void {
    process.stderr.write(`voice channel: ${message}\n`)
}
