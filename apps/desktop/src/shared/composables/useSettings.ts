import { inject, ref, type App, type InjectionKey, type Ref } from 'vue'
import * as api from '../api'
import type { Settings } from '../types'

const SETTINGS: InjectionKey<Ref<Settings>> = Symbol('settings')

/** Loads a window's settings before it mounts and keeps them current. */
export async function provideSettings(app: App): Promise<void> {
    const settings = ref(await api.getSettings())
    await api.onSettingsChanged(s => (settings.value = s))
    app.provide(SETTINGS, settings)
}

export function useSettings() {
    const settings = inject(SETTINGS)
    if (!settings) throw new Error('provideSettings must run before the window mounts')
    return { settings }
}
