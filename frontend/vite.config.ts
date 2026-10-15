import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    // Forward /api/* to the axum backend, so the browser sees a single origin
    // (no CORS needed, and no hardcoded localhost:8000 in components).
    proxy: {
      '/api': 'http://localhost:8000',
    },
  },
})
