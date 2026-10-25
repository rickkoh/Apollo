// The only module that talks to Tauri: one function per command, one `on…` per event.
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
    AboutInfo,
    CliStatus,
    HubState,
    HubStatus,
    Message,
    MessageId,
    MessageUpdated,
    OverlayVisibility,
    PanelStatus,
    PluginStatus,
    SessionId,
    SessionView,
    Settings,
    TargetChanged,
    UiError,
    UpdateStatus,
} from './types'

export type { UnlistenFn }

export function isUiError(value: unknown): value is UiError {
    return typeof value === 'object' && value !== null && 'code' in value && 'message' in value
}

export function errorMessage(err: unknown): string {
    return isUiError(err) ? err.message : String(err)
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    try {
        return await invoke<T>(command, args)
    } catch (err) {
        throw isUiError(err) ? err : ({ code: 'internal', message: String(err) } satisfies UiError)
    }
}

function on<T>(event: string, callback: (payload: T) => void): Promise<UnlistenFn> {
    return listen<T>(event, e => callback(e.payload))
}

export const hubStatus = () => call<HubStatus>('hub_status')
export const listSessions = () => call<SessionView[]>('list_sessions')
export const setTarget = (id: SessionId) => call<void>('set_target', { id })
export const send = (text: string) => call<Message>('send', { text })
export const history = (session: SessionId, limit: number, before?: MessageId) =>
    call<Message[]>('history', { session, before: before ?? null, limit })
export const recentMessages = (limit: number) => call<Message[]>('recent_messages', { limit })
export const markRead = (session: SessionId, upto: MessageId) => call<void>('mark_read', { session, upto })
export const renameSession = (id: SessionId, title: string | null) => call<void>('rename_session', { id, title })
export const forgetSession = (id: SessionId) => call<void>('forget_session', { id })
export const clearHistory = () => call<void>('clear_history')
export const getSettings = () => call<Settings>('get_settings')
export const updateSettings = (patch: object) => call<Settings>('update_settings', { patch })
export const overlayResize = (height: number) => call<void>('overlay_resize', { height })
export const overlayVisible = () => call<boolean>('overlay_visible')
export const setOverlayVisible = (visible: boolean) => call<void>('set_overlay_visible', { visible })
export const cliStatus = () => call<CliStatus>('cli_status')
export const installCli = () => call<CliStatus>('install_cli')
export const pluginStatus = () => call<PluginStatus>('plugin_status')
export const panelStatus = () => call<PanelStatus>('panel_status')
export const about = () => call<AboutInfo>('about')
export const revealDataFolder = () => call<void>('reveal_data_folder')
export const updateStatus = () => call<UpdateStatus | null>('update_status')
export const checkForUpdates = () => call<UpdateStatus>('check_for_updates')
export const installUpdate = () => call<void>('install_update')
export const openAtLogin = () => call<boolean | null>('open_at_login')
export const setOpenAtLogin = (enabled: boolean) => call<boolean>('set_open_at_login', { enabled })
export const openPanel = (route?: string) => call<void>('open_panel', { route: route ?? null })

export const onSessionsChanged = (cb: (sessions: SessionView[]) => void) => on('sessions-changed', cb)
export const onTargetChanged = (cb: (payload: TargetChanged) => void) => on('target-changed', cb)
export const onMessageAdded = (cb: (message: Message) => void) => on('message-added', cb)
export const onMessageUpdated = (cb: (payload: MessageUpdated) => void) => on('message-updated', cb)
export const onSettingsChanged = (cb: (settings: Settings) => void) => on('settings-changed', cb)
export const onOverlayVisibility = (cb: (payload: OverlayVisibility) => void) => on('overlay-visibility', cb)
export const onHubState = (cb: (payload: HubState) => void) => on('hub-state', cb)
export const onUpdateStatus = (cb: (status: UpdateStatus) => void) => on('update-status', cb)
export const onPanelNavigate = (cb: (route: string) => void) => on('panel-navigate', cb)
