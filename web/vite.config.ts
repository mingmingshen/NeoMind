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
    // Allow external access for mobile/LAN testing
    strictPort: false,
    host: '0.0.0.0',  // Bind to all interfaces for LAN access
    proxy: {
      '/api': {
        // Use environment variable or default to localhost
        // For LAN access, set VITE_API_TARGET=http://<server-ip>:9375
        target: process.env.VITE_API_TARGET || 'http://127.0.0.1:9375',
        changeOrigin: true,
        ws: true,
        // Increase timeout for large file uploads (5 minutes)
        timeout: 300000,
        // Increase proxy timeout for large uploads
        proxyTimeout: 300000,
        // Configure proxy to handle both localhost and LAN access
        configure: (proxy, _options) => {
          proxy.on('error', (err, req, _res) => {
            const code = (err as NodeJS.ErrnoException)?.code
            const isWs = req?.url?.includes('/api/events/ws') || req?.url?.includes('/api/chat')
            const isUpload = req?.url?.includes('/upload')
            const isExpectedWsClose = (code === 'ECONNRESET' || code === 'EPIPE') && isWs
            const isUploadError = (code === 'ECONNRESET' || code === 'EPIPE') && isUpload

            if (isExpectedWsClose) {
              // WebSocket closed (backend/client closed or proxy write after close) - expected, no need to log as error
              if (code === 'EPIPE') {
                console.warn('[Vite proxy] WebSocket write after close (EPIPE). Connection was closed by peer.')
              } else {
                console.warn('[Vite proxy] WebSocket connection closed by backend (ECONNRESET).')
              }
            } else if (isUploadError) {
              // Upload error - log with more context but don't spam
              console.warn('[Vite proxy] Upload connection error:', code, '- URL:', req?.url)
              console.warn('[Vite proxy] This may indicate the backend rejected the request (e.g., auth failure) or the connection was interrupted.')
            } else {
              console.error('[Vite proxy]', err)
            }
          })
          proxy.on('proxyReq', (proxyReq, req, _res) => {
            // For upload requests, log less verbosely and include content-length for debugging
            if (req?.url?.includes('/upload')) {
              const contentLength = req.headers['content-length']
              console.log('[Proxy Upload]', req.method, req.url, 'Content-Length:', contentLength || 'unknown')

              // Set longer timeout for upload requests on the underlying socket
              if (proxyReq.socket) {
                proxyReq.socket.setTimeout(300000)  // 5 minutes
              }
            } else {
              console.log('[Proxy]', req.method, req.url, '->', proxyReq.getHeader('host') + proxyReq.path)
            }
          })
          proxy.on('proxyReqWs', (proxyReq, req, socket, options, head) => {
            // Log WebSocket connection attempts
            console.log('[Proxy WS]', req.url, '->', options.target + req.url)

            // Set longer timeout for WebSocket connections
            socket.setTimeout(0)  // Disable timeout - let the app handle it with heartbeats

            // Handle WebSocket errors more gracefully
            socket.on('error', (err) => {
              const code = (err as NodeJS.ErrnoException)?.code
              // ECONNRESET is common when browser refreshes or navigates away
              if (code === 'ECONNRESET') {
                console.warn('[Proxy WS] Connection reset by client (normal during page refresh/navigation)')
              } else {
                console.error('[Proxy WS] Socket error:', err.message)
              }
            })

            socket.on('close', () => {
              console.log('[Proxy WS] Socket closed for', req.url)
            })
          })
          proxy.on('proxyRes', (proxyRes, req, _res) => {
            // Log upload response status for debugging
            if (req?.url?.includes('/upload')) {
              console.log('[Proxy Upload Response]', proxyRes.statusCode, req?.url)
            }
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
    // Performance: Enable sourcemaps for production debugging (optional)
    sourcemap: false,
    rollupOptions: {
      output: {
        // Optimized chunking strategy for better caching and smaller initial load
        manualChunks: (id) => {
          // 1. Separate large visualization libraries
          if (id.includes('node_modules/recharts') || id.includes('recharts')) {
            return 'vendor-recharts'
          }
          
          // 2. Separate UI component libraries
          if (id.includes('node_modules/@radix-ui')) {
            return 'vendor-radix'
          }
          if (id.includes('node_modules/lucide-react')) {
            return 'vendor-icons'
          }
          
          // 3. State management
          if (id.includes('node_modules/zustand')) {
            return 'vendor-state'
          }
          
          // 4. React core (shared across all pages)
          if (id.includes('node_modules/react') || id.includes('node_modules/react-dom')) {
            return 'vendor-react'
          }
          
          // 5. React Router
          if (id.includes('node_modules/react-router')) {
            return 'vendor-router'
          }
          
          // 6. Page-level code splitting
          if (id.includes('/pages/dashboard-components/VisualDashboard')) {
            return 'page-dashboard'
          }
          if (id.includes('/pages/agents')) {
            return 'page-agents'
          }
          if (id.includes('/pages/devices')) {
            return 'page-devices'
          }
          if (id.includes('/pages/automation')) {
            return 'page-automation'
          }
          if (id.includes('/pages/chat')) {
            return 'page-chat'
          }
          if (id.includes('/pages/messages')) {
            return 'page-messages'
          }
          if (id.includes('/pages/settings')) {
            return 'page-settings'
          }
          if (id.includes('/pages/login') || id.includes('/pages/setup')) {
            return 'page-auth'
          }
          
          // 7. Heavy components
          if (id.includes('node_modules/@codemirror')) {
            return 'vendor-editor'
          }
          if (id.includes('node_modules/react-markdown')) {
            return 'vendor-markdown'
          }
          
          // 8. All other node_modules
          if (id.includes('node_modules')) {
            return 'vendor-core'
          }
          
          return undefined
        },
      },
    },
  },
})
