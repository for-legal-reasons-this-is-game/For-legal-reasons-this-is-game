import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    // "@/components/ui/button" -> src/components/ui/button (same alias as in tsconfig)
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    // Forward /api/* to the axum backend, so the browser sees a single origin
    // (no CORS needed, and no hardcoded localhost:8000 in components).
    proxy: {
      '/api': 'http://localhost:8000',
    },
  },
})
