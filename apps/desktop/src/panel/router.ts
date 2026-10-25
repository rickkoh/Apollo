import { createRouter, createWebHashHistory } from 'vue-router'
import Home from './pages/Home.vue'
import SessionDetail from './pages/SessionDetail.vue'
import Sessions from './pages/Sessions.vue'
import Settings from './pages/Settings.vue'
import Shortcuts from './pages/Shortcuts.vue'

export const navigation = [
    { path: '/', label: 'Home', icon: '⌂' },
    { path: '/sessions', label: 'Sessions', icon: '☰' },
    { path: '/shortcuts', label: 'Shortcuts', icon: '⌘' },
] as const

export const router = createRouter({
    history: createWebHashHistory(),
    routes: [
        { path: '/', component: Home },
        { path: '/sessions', component: Sessions },
        { path: '/sessions/:id', component: SessionDetail, props: true },
        { path: '/shortcuts', component: Shortcuts },
        { path: '/settings', component: Settings },
    ],
})
