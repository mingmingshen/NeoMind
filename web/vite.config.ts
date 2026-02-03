import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import viteCompression from 'vite-plugin-compression'

export default defineConfig({
  plugins: [
    react({
      // Faster development builds
      babel: {
        plugins: process.env.NODE_ENV === 'development' ? [] : undefined
      }
    }),
    // Performance: Generate gzip and brotli compressed assets
    viteCompression({
      algorithm: 'gzip',
      ext: '.gz',
      threshold: 10240, // Only compress files larger than 10KB
    }),
    viteCompression({
      algorithm: 'brotliCompress',
      ext: '.br',
      threshold: 10240,
    }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  optimizeDeps: {
    // Pre-bundle heavy dependencies for faster dev start
    include: [
      'react',
      'react-dom',
      'react-router-dom',
      'zustand',
      'recharts',
      '@radix-ui/react-dialog',
      '@radix-ui/react-dropdown-menu',
    ],
  },
  server: {
    port: 5173,
    // Faster HMR
    hmr: {
      overlay: true
    },
    proxy: {
      '/api': {
        target: 'http://localhost:9375',
        changeOrigin: true,
        ws: true,
        // Only proxy /api requests, not static files
        configure: (proxy, _options) => {
          proxy.on('error', (err, _req, _res) => {
            console.log('proxy error', err)
          })
          proxy.on('proxyReq', (proxyReq, req, _res) => {
            console.log('[Proxy]', req.method, req.url, '->', proxyReq.getHeader('host') + proxyReq.path)
          })
        },
      },
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    chunkSizeWarningLimit: 500,
    // Performance: Use esbuild for faster minification
    minify: 'esbuild',
    target: 'es2020',
    rollupOptions: {
      output: {
        // Simplified chunking strategy to avoid ALL circular dependencies
        // Put all node_modules in a single vendor chunk
        manualChunks: (id) => {
          // Application chunks - only split heavy pages
          if (id.includes('/pages/login')) return 'page-login'
          if (id.includes('/pages/setup')) return 'page-setup'
          if (id.includes('/pages/dashboard-components/VisualDashboard')) return 'page-dashboard'
          if (id.includes('/pages/plugins')) return 'page-plugins'
          if (id.includes('/pages/agents') || id.includes('/pages/agents-components')) return 'page-agents'
          if (id.includes('/pages/devices')) return 'page-devices'
          if (id.includes('/pages/automation')) return 'page-automation'
          if (id.includes('/pages/events')) return 'page-events'
          if (id.includes('/pages/commands')) return 'page-commands'
          if (id.includes('/pages/decisions')) return 'page-decisions'
          if (id.includes('/pages/messages')) return 'page-messages'
          if (id.includes('/pages/settings')) return 'page-settings'
          if (id.includes('/pages/chat')) return 'page-chat'

          // ALL node_modules in one chunk to eliminate circular dependencies
          if (id.includes('node_modules')) {
            return 'vendor'
          }

          return undefined
        },
      },
    },
  },
})
