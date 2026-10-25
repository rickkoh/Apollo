import { fileURLToPath, URL } from 'node:url'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
    plugins: [vue()],
    clearScreen: false,
    resolve: {
        alias: { '@types': fileURLToPath(new URL('../../shared/types', import.meta.url)) },
    },
    server: {
        port: 1420,
        strictPort: true,
        host: '127.0.0.1',
        fs: { allow: ['..', '../../shared'] },
    },
    build: {
        target: 'safari16',
        rollupOptions: {
            input: {
                overlay: fileURLToPath(new URL('overlay.html', import.meta.url)),
                panel: fileURLToPath(new URL('panel.html', import.meta.url)),
            },
        },
    },
})
