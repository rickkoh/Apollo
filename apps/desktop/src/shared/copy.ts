// Text the UI shows in more than one place (D97); an error from the shell shows its own `UiError` message.

export const HUB_DOWN = "Apollo isn't running its hub. Restart the app"

/** The words around the launcher command, so `StartSessionHint` can set the command in code. */
export const START_SESSION = { before: 'Start a session with', after: "in any terminal, Cursor's included" } as const

export function startSession(command: string): string {
    return `${START_SESSION.before} ${command} ${START_SESSION.after}`
}
