import { createRouter, createWebHashHistory, type Router } from 'vue-router'
import type { IconName } from '../shared/components/Icon.vue'
import Home from './pages/Home.vue'
import SessionDetail from './pages/SessionDetail.vue'
import Sessions from './pages/Sessions.vue'
import Shortcuts from './pages/Shortcuts.vue'

export const navigation: readonly { path: string; label: string; icon: IconName }[] = [
    { path: '/', label: 'Home', icon: 'home' },
    { path: '/sessions', label: 'Sessions', icon: 'sessions' },
    { path: '/shortcuts', label: 'Shortcuts', icon: 'shortcuts' },
]

export const router: Router = createRouter({
    history: createWebHashHistory(),
    routes: [
        { path: '/', component: Home },
        { path: '/sessions', component: Sessions },
        { path: '/sessions/:id', component: SessionDetail, props: true, meta: { fill: true } },
        { path: '/shortcuts', component: Shortcuts },
        {
            path: '/settings/:section?',
            redirect: to => ({
                path: router.currentRoute.value.path,
                query: { settings: String(to.params.section || 'general') },
            }),
        },
    ],
})
