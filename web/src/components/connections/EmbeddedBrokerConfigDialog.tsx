import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useErrorHandler } from '@/hooks/useErrorHandler'
import {
  Settings,
  Trash2,
  Plus,
  Loader2,
  AlertTriangle,
} from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogContentBody,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Badge } from '@/components/ui/badge'
import { Textarea } from '@/components/ui/textarea'
import { api } from '@/lib/api'
import { useToast } from '@/hooks/use-toast'
import { cn } from '@/lib/utils'

interface BrokerConfig {
  listen: string
  port: number
  max_connections: number
  auth_enabled: boolean
  credentials: { username: string; password: string }[]
  tls_enabled: boolean
  tls_cert_path: string | null
  tls_key_path: string | null
  tls_ca_path: string | null
}

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function EmbeddedBrokerConfigDialog({ open, onOpenChange }: Props) {
  const { t } = useTranslation('settings')
  const { toast } = useToast()
  const { handleError } = useErrorHandler()

  const [config, setConfig] = useState<BrokerConfig | null>(null)
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)

  // Form state for general settings
  const [listen, setListen] = useState('')
  const [port, setPort] = useState(1883)

  // Form state for TLS certificates
  const [certPem, setCertPem] = useState('')
  const [keyPem, setKeyPem] = useState('')
  const [caPem, setCaPem] = useState('')

  // Form state for adding credential
  const [showAddCredential, setShowAddCredential] = useState(false)
  const [newUsername, setNewUsername] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [addingCredential, setAddingCredential] = useState(false)

  // Track if config has unsaved changes that require restart
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false)

  useEffect(() => {
    if (open) {
      loadConfig()
    }
  }, [open])

  const loadConfig = async () => {
    setLoading(true)
    try {
      const data = await api.getEmbeddedBrokerConfig()
      setConfig(data)
      setListen(data.listen)
      setPort(data.port)
      setCertPem('')
      setKeyPem('')
      setCaPem('')
      setHasUnsavedChanges(false)
    } catch (error) {
      handleError(error)
    } finally {
      setLoading(false)
    }
  }

  const handleToggleAuth = async (enabled: boolean) => {
    if (!config) return
    try {
      await api.updateEmbeddedBrokerConfig({ auth_enabled: enabled })
      setConfig({ ...config, auth_enabled: enabled })
      toast({
        title: enabled ? t('broker.authEnabled') : t('broker.authDisabled'),
        description: t('broker.authUpdateSuccess'),
      })
    } catch (error) {
      handleError(error)
    }
  }

  const handleToggleTls = async (enabled: boolean) => {
    if (!config) return
    try {
      await api.updateEmbeddedBrokerConfig({ tls_enabled: enabled })
      setConfig({ ...config, tls_enabled: enabled })
      setHasUnsavedChanges(true)
      toast({
        title: enabled ? t('broker.tlsEnabled') : t('broker.tlsDisabled'),
        description: t('broker.restartWarning'),
      })
    } catch (error) {
      handleError(error)
    }
  }

  const handleAddCredential = async () => {
    if (!newUsername.trim() || !newPassword.trim()) {
      toast({
        title: t('broker.invalidInput'),
        description: t('broker.usernamePasswordRequired'),
        variant: 'destructive',
      })
      return
    }

    setAddingCredential(true)
    try {
      await api.addMqttCredential(newUsername.trim(), newPassword)
      await loadConfig()
      setNewUsername('')
      setNewPassword('')
      setShowAddCredential(false)
      toast({
        title: t('broker.userAdded'),
        description: t('broker.userAddSuccess', { username: newUsername }),
      })
    } catch (error) {
      handleError(error)
    } finally {
      setAddingCredential(false)
    }
  }

  const handleDeleteCredential = async (username: string) => {
    try {
      await api.deleteMqttCredential(username)
      await loadConfig()
      toast({
        title: t('broker.userDeleted'),
        description: t('broker.userDeleteSuccess', { username }),
      })
    } catch (error) {
      handleError(error)
    }
  }

  const handleSave = async () => {
    if (!config) return
    setSaving(true)
    try {
      // Update basic settings
      await api.updateEmbeddedBrokerConfig({
        listen,
        port,
      })

      // Upload TLS certificates if provided
      if (certPem && keyPem) {
        await api.uploadMqttTlsCert(certPem, keyPem, caPem || undefined)
      }

      await loadConfig()
      setHasUnsavedChanges(false)
      onOpenChange(false)
      toast({
        title: t('broker.configSaved'),
        description: t('broker.restartRequired'),
      })
    } catch (error) {
      handleError(error)
    } finally {
      setSaving(false)
    }
  }

  if (loading) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-[600px]">
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        </DialogContent>
      </Dialog>
    )
  }

  if (!config) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t('broker.settings')}</DialogTitle>
          <DialogDescription>
            {t('broker.settingsDescription')}
          </DialogDescription>
        </DialogHeader>

        <DialogContentBody className="space-y-6">
          {/* General Settings */}
          <div className="space-y-4">
            <h3 className="text-sm font-medium">{t('broker.general')}</h3>
            <div className="grid gap-4">
              <div className="grid gap-2">
                <Label htmlFor="listen">{t('broker.listen')}</Label>
                <Input
                  id="listen"
                  value={listen}
                  onChange={(e) => {
                    setListen(e.target.value)
                    setHasUnsavedChanges(true)
                  }}
                  placeholder="0.0.0.0"
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="port">{t('broker.port')}</Label>
                <Input
                  id="port"
                  type="number"
                  min={1}
                  max={65535}
                  value={port}
                  onChange={(e) => {
                    setPort(Number(e.target.value))
                    setHasUnsavedChanges(true)
                  }}
                />
              </div>
              <div className="text-xs text-muted-foreground">
                {t('broker.maxConnections')}: {config.max_connections}
              </div>
            </div>
          </div>

          {/* Authentication */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-medium">{t('broker.auth')}</h3>
              <Switch
                checked={config.auth_enabled}
                onCheckedChange={handleToggleAuth}
              />
            </div>

            {config.auth_enabled && (
              <div className="space-y-2">
                {config.credentials.map((cred) => (
                  <div
                    key={cred.username}
                    className="flex items-center justify-between p-2 rounded border bg-muted-30"
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-mono">{cred.username}</span>
                      <Badge variant="secondary" className="text-xs">
                        ••••••••
                      </Badge>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-8 w-8 p-0 text-destructive hover:text-destructive"
                      onClick={() => handleDeleteCredential(cred.username)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                ))}

                {showAddCredential ? (
                  <div className="space-y-2 p-3 rounded border bg-background">
                    <div className="grid gap-2">
                      <Input
                        placeholder={t('broker.username')}
                        value={newUsername}
                        onChange={(e) => setNewUsername(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleAddCredential()
                        }}
                      />
                      <Input
                        type="password"
                        placeholder={t('broker.password')}
                        value={newPassword}
                        onChange={(e) => setNewPassword(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleAddCredential()
                        }}
                      />
                    </div>
                    <div className="flex gap-2">
                      <Button
                        size="sm"
                        onClick={handleAddCredential}
                        disabled={addingCredential}
                      >
                        {addingCredential ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          t('broker.add')
                        )}
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => {
                          setShowAddCredential(false)
                          setNewUsername('')
                          setNewPassword('')
                        }}
                      >
                        {t('broker.cancel')}
                      </Button>
                    </div>
                  </div>
                ) : (
                  <Button
                    variant="outline"
                    size="sm"
                    className="w-full"
                    onClick={() => setShowAddCredential(true)}
                  >
                    <Plus className="h-4 w-4 mr-2" />
                    {t('broker.addUser')}
                  </Button>
                )}
              </div>
            )}
          </div>

          {/* TLS */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-medium">{t('broker.tls')}</h3>
              <Switch
                checked={config.tls_enabled}
                onCheckedChange={handleToggleTls}
              />
            </div>

            {config.tls_enabled && (
              <div className="space-y-3">
                <div className="grid gap-2">
                  <Label htmlFor="cert">{t('broker.serverCert')}</Label>
                  <Textarea
                    id="cert"
                    placeholder="-----BEGIN CERTIFICATE-----&#10;...&#10;-----END CERTIFICATE-----"
                    value={certPem}
                    onChange={(e) => {
                      setCertPem(e.target.value)
                      setHasUnsavedChanges(true)
                    }}
                    rows={4}
                    className="font-mono text-xs"
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="key">{t('broker.privateKey')}</Label>
                  <Textarea
                    id="key"
                    placeholder="-----BEGIN PRIVATE KEY-----&#10;...&#10;-----END PRIVATE KEY-----"
                    value={keyPem}
                    onChange={(e) => {
                      setKeyPem(e.target.value)
                      setHasUnsavedChanges(true)
                    }}
                    rows={4}
                    className="font-mono text-xs"
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="ca">{t('broker.caCert')} ({t('broker.optional')})</Label>
                  <Textarea
                    id="ca"
                    placeholder="-----BEGIN CERTIFICATE-----&#10;...&#10;-----END CERTIFICATE-----"
                    value={caPem}
                    onChange={(e) => {
                      setCaPem(e.target.value)
                      setHasUnsavedChanges(true)
                    }}
                    rows={4}
                    className="font-mono text-xs"
                  />
                </div>
                {config.tls_cert_path && (
                  <div className="text-xs text-muted-foreground">
                    {t('broker.currentCert')}: {config.tls_cert_path}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Warning */}
          {hasUnsavedChanges && (
            <div className="flex items-start gap-2 p-3 rounded bg-warning-light text-warning dark:bg-warning-light dark:text-warning">
              <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
              <p className="text-sm">{t('broker.restartWarning')}</p>
            </div>
          )}
        </DialogContentBody>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('broker.cancel')}
          </Button>
          <Button onClick={handleSave} disabled={saving}>
            {saving ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                {t('broker.saving')}
              </>
            ) : (
              t('broker.save')
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
