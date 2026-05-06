import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { useNavigate } from "react-router-dom"
import { useStore } from "@/store"
import { Languages, Lock, User, Shield, ArrowLeft, Server, Globe, ChevronRight, Check, KeyRound } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { Input } from "@/components/ui/input"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { BrandLogoHorizontal } from "@/components/shared/BrandName"
import { forceViewportReset } from "@/hooks/useVisualViewport"
import { tokenManager, getApiBase, getApiKey, setApiBase, clearApiKey, setApiKey } from "@/lib/api"
import { INSTANCE_CACHE_KEY, CURRENT_INSTANCE_KEY, PENDING_SWITCH_KEY } from "@/lib/instance-constants"
import { decryptApiKey } from "@/store/slices/instanceSlice"

const languages = [
  { code: 'en', name: 'English' },
  { code: 'zh', name: '简体中文' },
]

// LocalStorage keys for remembering credentials
const CREDENTIALS_KEY = 'neomind_remembered_credentials'

interface CachedInstance {
  id: string
  name: string
  url: string
  api_key?: string
  encrypted_key?: string
  is_local: boolean
  last_status: string
}

function getCachedInstances(): CachedInstance[] {
  try {
    const raw = localStorage.getItem(INSTANCE_CACHE_KEY)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

// Error translation helper
function translateError(error: string, t: (key: string, params?: Record<string, unknown>) => string): string {
  const lowerError = error.toLowerCase()
  if (lowerError.includes("invalid username or password") || lowerError.includes("invalid credentials")) {
    return t("invalidCredentials")
  }
  if (lowerError.includes("user not found")) {
    return t("userNotFound")
  }
  if (lowerError.includes("user disabled") || lowerError.includes("account is disabled")) {
    return t("accountDisabled")
  }
  if (lowerError.includes("password must be at least")) {
    return t("minPasswordLength", { ns: 'validation' })
  }
  if (lowerError.includes("username must be at least")) {
    return t("minUsernameLength", { ns: 'validation' })
  }
  if (lowerError.includes("user already exists")) {
    return t("userAlreadyExists")
  }
  if (lowerError.includes("unauthorized")) {
    return t("authFailed")
  }
  return error || t("loginFailed")
}

export function LoginPage() {
  const { t, i18n } = useTranslation(['common', 'auth', 'instances'])
  const { login, checkAuthStatus } = useStore()
  const navigate = useNavigate()
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [rememberMe, setRememberMe] = useState(false)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState("")
  const [isFirstSetup, setIsFirstSetup] = useState<boolean | null>(null)
  const [checkingAuth, setCheckingAuth] = useState(true)
  const [hasLoadedCredentials, setHasLoadedCredentials] = useState(false)
  const [showInstancePicker, setShowInstancePicker] = useState(false)

  const cachedInstances = getCachedInstances()
  const apiBase = getApiBase()
  const isRemote = !!(apiBase && apiBase !== '/api' && !apiBase.includes('localhost') && !apiBase.includes('127.0.0.1'))

  // Handle instance switch — use encrypted_key from backend
  const handleInstanceSwitch = (instance: CachedInstance) => {
    const fullKey = instance.encrypted_key ? decryptApiKey(instance.encrypted_key) : ''
    localStorage.setItem(CURRENT_INSTANCE_KEY, instance.id)
    localStorage.setItem(PENDING_SWITCH_KEY, JSON.stringify({
      targetId: instance.id,
      previousId: 'local-default',
      apiUrl: instance.is_local ? '' : `${instance.url}/api`,
      apiKey: instance.is_local ? '' : fullKey,
    }))
    window.location.reload()
  }

  const handleBackToLocal = () => {
    localStorage.setItem(CURRENT_INSTANCE_KEY, 'local-default')
    setApiBase('')
    clearApiKey()
    window.location.reload()
  }

  // Check if already authenticated on mount
  useEffect(() => {
    const checkExistingAuth = async () => {
      const token = tokenManager.getToken()
      const apiKey = getApiKey()
      if (token) {
        try {
          await checkAuthStatus()
          navigate('/', { replace: true })
          return
        } catch {
          tokenManager.clearToken()
        }
      } else if (apiKey) {
        checkAuthStatus()
        navigate('/', { replace: true })
        return
      }
      setCheckingAuth(false)
    }
    checkExistingAuth()
  }, [checkAuthStatus, navigate])

  // Load saved credentials on mount
  useEffect(() => {
    if (checkingAuth) return

    const checkSetupStatus = async () => {
      const apiBase = getApiBase()
      try {
        const response = await fetch(`${apiBase}/setup/status`, {
          signal: AbortSignal.timeout(5000),
        })
        if (response.ok) {
          const data = await response.json() as { setup_required: boolean }
          setIsFirstSetup(data.setup_required)
          if (data.setup_required) {
            navigate('/setup', { replace: true })
          }
        } else {
          setIsFirstSetup(false)
        }
      } catch {
        setIsFirstSetup(false)
      }
    }
    checkSetupStatus()

    try {
      const saved = localStorage.getItem(CREDENTIALS_KEY)
      if (saved) {
        const credentials = JSON.parse(saved)
        if (credentials.username) setUsername(credentials.username)
        if (credentials.password) setPassword(credentials.password)
        if (credentials.rememberMe !== undefined) setRememberMe(credentials.rememberMe)
        setHasLoadedCredentials(true)
      }
    } catch { /* ignore */ }

    const localToken = localStorage.getItem('neomind_token')
    if (localToken) {
      ;(async () => {
        try {
          await checkAuthStatus()
          navigate('/', { replace: true })
        } catch {
          tokenManager.clearToken()
        }
      })()
    }
  }, [checkingAuth, navigate])

  // Save credentials when rememberMe changes
  useEffect(() => {
    if (!hasLoadedCredentials) return
    try {
      if (rememberMe) {
        const saved = localStorage.getItem(CREDENTIALS_KEY)
        const existing = saved ? JSON.parse(saved) : {}
        localStorage.setItem(CREDENTIALS_KEY, JSON.stringify({
          username: username || existing.username,
          password: existing.password || '',
          rememberMe: true,
        }))
      } else {
        localStorage.removeItem(CREDENTIALS_KEY)
      }
    } catch { /* ignore */ }
  }, [rememberMe, username, hasLoadedCredentials])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError("")
    setIsLoading(true)

    try {
      await login(username, password, rememberMe)
      if (rememberMe) {
        try { localStorage.setItem(CREDENTIALS_KEY, JSON.stringify({ username, password, rememberMe: true })) } catch { /* ignore */ }
      } else {
        try { localStorage.removeItem(CREDENTIALS_KEY) } catch { /* ignore */ }
      }
      setHasLoadedCredentials(true)
      forceViewportReset()
      navigate('/', { replace: true })
    } catch (err) {
      setError(translateError(err instanceof Error ? err.message : String(t('auth:loginFailed')), t))
      forceViewportReset()
    } finally {
      setIsLoading(false)
    }
  }

  const handleBackdropClick = () => {
    forceViewportReset()
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur()
  }

  // Show loading while checking authentication
  if (checkingAuth) {
    return (
      <div className="flex flex-col bg-background relative overflow-hidden viewport-full items-center justify-center">
        <div className="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin" />
      </div>
    )
  }

  // Full-screen instance picker
  if (showInstancePicker) {
    return (
      <div className="flex flex-col bg-background viewport-full">
        {/* Header */}
        <header className="flex items-center gap-3 px-4 sm:px-6 h-14 border-b border-border">
          <Button variant="ghost" size="sm" onClick={() => setShowInstancePicker(false)}>
            <ArrowLeft className="h-4 w-4 mr-1" />
            {t('common:back')}
          </Button>
          <h2 className="text-lg font-semibold">{t('instances:selectBackend')}</h2>
        </header>

        {/* Instance List */}
        <div className="flex-1 overflow-auto px-4 py-4">
          <div className="max-w-lg mx-auto space-y-3">
            {cachedInstances.map((inst) => {
              const isCurrent = inst.id === localStorage.getItem(CURRENT_INSTANCE_KEY)
              const hasApiKey = !!(inst.encrypted_key || inst.api_key)
              return (
                <button
                  key={inst.id}
                  onClick={() => { setShowInstancePicker(false); handleInstanceSwitch(inst) }}
                  className={`w-full flex items-center gap-4 p-4 rounded-xl bg-bg-50 border transition-colors text-left ${
                    isCurrent ? 'border-primary' : 'border-border hover:border-primary'
                  }`}
                >
                  <div className={`flex-shrink-0 w-10 h-10 rounded-full flex items-center justify-center ${isCurrent ? 'bg-primary/10' : 'bg-muted'}`}>
                    {inst.is_local ? <Globe className="h-5 w-5 text-primary" /> : <Server className={`h-5 w-5 ${isCurrent ? 'text-primary' : 'text-accent-cyan'}`} />}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium truncate flex items-center gap-2">
                      {inst.is_local ? t('instances:localBackend') : inst.name}
                      {isCurrent && (
                        <span className="inline-flex items-center gap-0.5 text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-primary/10 text-primary">
                          <Check className="h-3 w-3" />
                          {t('instances:current')}
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-muted-foreground truncate flex items-center gap-2 mt-0.5">
                      <span>{inst.is_local ? 'localhost:9375' : inst.url.replace(/^https?:\/\//, '')}</span>
                      <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-muted">
                        <KeyRound className="h-3 w-3" />
                        {inst.is_local ? t('instances:authUserLogin') : hasApiKey ? t('instances:authApiKey') : t('instances:authUserLogin')}
                      </span>
                    </div>
                  </div>
                  <ChevronRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                </button>
              )
            })}
            {cachedInstances.length === 0 && (
              <p className="text-center text-muted-foreground py-12">{t('instances:noInstances')}</p>
            )}
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="flex flex-col bg-background relative overflow-hidden viewport-full">
      {/* Background Effects */}
      <div className="fixed inset-0">
        <div className="absolute inset-0 bg-gradient-to-br from-background via-background to-muted" />
        <div className="absolute inset-0" style={{
          backgroundImage: 'radial-gradient(circle, hsl(var(--border) / 0.1) 1px, transparent 1px)',
          backgroundSize: '32px 32px'
        }} />
        <svg className="absolute inset-0 w-full h-full opacity-[0.03]" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <pattern id="network-grid" width="120" height="120" patternUnits="userSpaceOnUse">
              <circle cx="60" cy="60" r="1.5" fill="currentColor" />
              <line x1="60" y1="0" x2="60" y2="120" stroke="currentColor" strokeWidth="0.5" />
              <line x1="0" y1="60" x2="120" y2="60" stroke="currentColor" strokeWidth="0.5" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#network-grid)" />
        </svg>
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-muted rounded-full blur-3xl animate-pulse" style={{ animationDuration: '8s' }} />
        <div className="absolute top-[15%] left-[10%] w-32 h-32 bg-info-light rounded-full blur-2xl animate-pulse" style={{ animationDuration: '6s', animationDelay: '0s' }} />
        <div className="absolute bottom-[20%] right-[15%] w-40 h-40 bg-accent-purple-light rounded-full blur-2xl animate-pulse" style={{ animationDuration: '7s', animationDelay: '1s' }} />
        <div className="absolute top-[30%] right-[20%] w-24 h-24 bg-accent-cyan-light rounded-full blur-2xl animate-pulse" style={{ animationDuration: '5s', animationDelay: '2s' }} />
        <div className="absolute bottom-[30%] left-[20%] w-28 h-28 bg-accent-indigo-light rounded-full blur-2xl animate-pulse" style={{ animationDuration: '6s', animationDelay: '3s' }} />
        <div className="absolute inset-0 opacity-[0.02]">
          <div className="absolute top-0 left-1/4 w-px h-full bg-gradient-to-b from-transparent via-primary to-transparent" />
          <div className="absolute top-0 right-1/4 w-px h-full bg-gradient-to-b from-transparent via-blue-500 to-transparent" />
          <div className="absolute top-1/4 left-0 w-full h-px bg-gradient-to-r from-transparent via-accent-purple to-transparent" />
          <div className="absolute bottom-1/4 left-0 w-full h-px bg-gradient-to-r from-transparent via-accent-cyan to-transparent" />
        </div>
      </div>

      {/* Top Header */}
      <header className="absolute top-0 left-0 right-0 z-50 safe-top">
        <div className="flex items-center justify-between px-4 sm:px-6 h-14 sm:h-16">
          <div className="flex items-center gap-2 sm:gap-3">
            <BrandLogoHorizontal className="h-6 sm:h-7" />
          </div>
          <div className="flex items-center gap-1">
            {/* Backend switcher — always opens instance picker */}
            {cachedInstances.length > 0 ? (() => {
              const currentId = localStorage.getItem(CURRENT_INSTANCE_KEY)
              const current = cachedInstances.find(i => i.id === currentId)
              const label = current
                ? (current.is_local ? t('instances:localBackend') : current.name)
                : t('instances:localBackend')
              return (
                <Button variant="ghost" size="sm" className="gap-1 px-2 sm:px-3" onClick={() => setShowInstancePicker(true)}>
                  <Server className="h-4 w-4" />
                  <span className="hidden sm:inline">{label}</span>
                </Button>
              )
            })() : null}

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="gap-1 px-2 sm:px-3">
                  <Languages className="h-4 w-4" />
                  <span>{languages.find(l => l.code === i18n.language)?.name || 'Language'}</span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="min-w-[130px]">
                {languages.map((lang) => (
                  <DropdownMenuItem
                    key={lang.code}
                    onClick={() => i18n.changeLanguage(lang.code)}
                    className={i18n.language === lang.code ? 'bg-muted' : ''}
                  >
                    {lang.name}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main
        className="flex-1 px-4 sm:px-6 safe-bottom flex items-center justify-center min-h-0"
        onClick={(e) => {
          if ((e.target as HTMLElement).closest('form, button, a')) return
          handleBackdropClick()
        }}
      >
        <div className="w-full max-w-md">
          <div className="bg-bg-50 backdrop-blur-md rounded-lg p-6 sm:p-8">
            <h2 className="text-2xl sm:text-3xl font-semibold mb-4 sm:mb-6 text-center">{t('auth:login')}</h2>
            <form onSubmit={handleSubmit} className="flex flex-col gap-4 sm:gap-5">
              <div className="relative">
                <User className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                <Input
                  id="username"
                  type="text"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder={t('auth:username')}
                  autoComplete="username"
                  required
                  className="pl-9 h-11 bg-bg-70 border-border focus:bg-background dark:focus:bg-bg-50 focus:border-primary transition-colors text-base scroll-mb-32"
                />
              </div>
              <div className="relative">
                <Lock className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                <Input
                  id="password"
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder={t('auth:password')}
                  autoComplete="current-password"
                  required
                  className="pl-9 h-11 bg-bg-70 border-border focus:bg-background dark:focus:bg-bg-50 focus:border-primary transition-colors text-base scroll-mb-32"
                />
              </div>
              <label className="flex items-center gap-2 cursor-pointer group">
                <Checkbox
                  id="remember"
                  checked={rememberMe}
                  onCheckedChange={(checked) => setRememberMe(!!checked)}
                />
                <span className="text-sm text-muted-foreground group-hover:text-foreground transition-colors leading-none">
                  {t('auth:rememberMe')}
                </span>
              </label>
              {error && (
                <div className="flex items-start gap-2 text-sm text-destructive bg-muted rounded-md p-3">
                  <Shield className="h-4 w-4 mt-0.5 flex-shrink-0" />
                  <span>{error}</span>
                </div>
              )}
              <Button
                type="submit"
                disabled={isLoading || !username || !password}
                className="h-11 w-full"
                size="default"
              >
                {isLoading ? t('auth:loggingIn') : t('auth:login')}
              </Button>
            </form>
          </div>
        </div>
        <footer className="hidden sm:block absolute left-0 right-0 z-10 text-center bottom-6">
          <p className="text-xs text-muted-foreground">
            © CamThink {new Date().getFullYear()}
          </p>
        </footer>
      </main>
    </div>
  )
}
