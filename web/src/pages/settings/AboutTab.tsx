import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import {
  Bot,
  Cpu,
  Database,
  Network,
  Shield,
  Zap,
  Code,
  Heart,
  Book,
  Workflow,
  Layers,
  Monitor,
  HardDrive,
  Globe,
  Clock,
  Server,
} from "lucide-react"
import { api } from "@/lib/api"

interface SystemInfo {
  version: string
  uptime: number
  platform: string
  arch: string
  cpu_count: number
  total_memory: number
  used_memory: number
  free_memory: number
  available_memory: number
}

export function AboutTab() {
  const { t } = useTranslation(["common", "settings"])
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadSystemInfo()
  }, [])

  const loadSystemInfo = async () => {
    try {
      const response = await api.getSystemStats()
      setSystemInfo(response)
    } catch (e) {
      console.error("Failed to load system info:", e)
    } finally {
      setLoading(false)
    }
  }

  const formatBytes = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024)
    return gb.toFixed(2) + " GB"
  }

  const formatUptime = (seconds: number) => {
    const days = Math.floor(seconds / 86400)
    const hours = Math.floor((seconds % 86400) / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)

    if (days > 0) {
      return `${days}d ${hours}h ${minutes}m`
    }
    if (hours > 0) {
      return `${hours}h ${minutes}m`
    }
    return `${minutes}m`
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="text-center space-y-2">
        <div className="flex items-center justify-center gap-3">
          <div className="flex items-center justify-center w-16 h-16 rounded-2xl bg-gradient-to-br from-gray-900 to-gray-700 dark:from-white dark:to-gray-300 text-white dark:text-gray-900 shadow-lg">
            <Bot className="w-8 h-8" />
          </div>
          <h1 className="text-3xl font-bold bg-gradient-to-r from-gray-900 to-gray-600 dark:from-white dark:to-gray-400 bg-clip-text text-transparent">
            NeoTalk
          </h1>
        </div>
        <p className="text-muted-foreground">
          {t("settings:aboutDesc")}
        </p>
        <Badge variant="outline" className="mx-auto">
          {systemInfo?.version || "v0.1.0"}
        </Badge>
      </div>

      {/* System Info */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Monitor className="h-5 w-5 text-blue-500" />
            {t("settings:systemInfo")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="text-center py-8 text-muted-foreground text-sm">
              {t("common:loading")}
            </div>
          ) : systemInfo ? (
            <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
              <div className="p-3 bg-muted/50 rounded-lg">
                <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                  <Server className="h-3 w-3" />
                  {t("settings:platform")}
                </div>
                <div className="text-sm font-medium">
                  {systemInfo.platform} {systemInfo.arch}
                </div>
              </div>
              <div className="p-3 bg-muted/50 rounded-lg">
                <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                  <Clock className="h-3 w-3" />
                  {t("settings:uptime")}
                </div>
                <div className="text-sm font-medium">
                  {formatUptime(systemInfo.uptime)}
                </div>
              </div>
              <div className="p-3 bg-muted/50 rounded-lg">
                <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                  <Cpu className="h-3 w-3" />
                  {t("settings:cpuCores")}
                </div>
                <div className="text-sm font-medium">
                  {systemInfo.cpu_count} {t("settings:cores")}
                </div>
              </div>
              <div className="p-3 bg-muted/50 rounded-lg">
                <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                  <HardDrive className="h-3 w-3" />
                  {t("settings:totalMemory")}
                </div>
                <div className="text-sm font-medium">
                  {formatBytes(systemInfo.total_memory)}
                </div>
              </div>
              <div className="p-3 bg-muted/50 rounded-lg">
                <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                  <Database className="h-3 w-3" />
                  {t("settings:usedMemory")}
                </div>
                <div className="text-sm font-medium">
                  {formatBytes(systemInfo.used_memory)}
                </div>
              </div>
              <div className="p-3 bg-muted/50 rounded-lg">
                <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                  <Layers className="h-3 w-3" />
                  {t("settings:freeMemory")}
                </div>
                <div className="text-sm font-medium">
                  {formatBytes(systemInfo.available_memory)}
                </div>
              </div>
            </div>
          ) : (
            <div className="text-center py-8 text-muted-foreground text-sm">
              {t("settings:systemInfoUnavailable")}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Overview */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Heart className="h-5 w-5 text-red-500" />
            {t("settings:projectIntro")}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm">
          <p>{t("settings:aboutDesc1")}</p>
          <p>{t("settings:aboutDesc2")}</p>
        </CardContent>
      </Card>

      {/* Tech Stack */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Layers className="h-5 w-5 text-blue-500" />
            {t("settings:techStack")}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <h4 className="font-medium mb-2 flex items-center gap-2">
                <Cpu className="h-4 w-4 text-orange-500" />
                {t("settings:backend")}
              </h4>
              <div className="text-sm text-muted-foreground space-y-1">
                <div>• Rust - {t("settings:systemCore")}</div>
                <div>• Axum - Web {t("settings:frameWork")}</div>
                <div>• Tokio - {t("settings:asyncRuntime")}</div>
                <div>• redb - {t("settings:embeddedDB")}</div>
              </div>
            </div>
            <div>
              <h4 className="font-medium mb-2 flex items-center gap-2">
                <Code className="h-4 w-4 text-blue-500" />
                {t("settings:frontend")}
              </h4>
              <div className="text-sm text-muted-foreground space-y-1">
                <div>• React 18 - UI {t("settings:frameWork")}</div>
                <div>• TypeScript - {t("settings:typeSafe")}</div>
                <div>• Vite - {t("settings:buildTool")}</div>
                <div>• Tailwind CSS - {t("settings:styling")}</div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Core Features */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Zap className="h-5 w-5 text-warning" />
            {t("settings:coreFeatures")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <FeatureItem
              icon={<Bot className="h-4 w-4" />}
              title={t("settings:multiModelLLM")}
              description="Ollama、OpenAI、Anthropic、Google、xAI"
            />
            <FeatureItem
              icon={<Workflow className="h-4 w-4" />}
              title={t("settings:eventDriven")}
              description={`EventBus ${t("settings:realtimeMessaging")}`}
            />
            <FeatureItem
              icon={<Network className="h-4 w-4" />}
              title={t("settings:deviceManagement")}
              description={`MQTT、Modbus、Home Assistant ${t("settings:integration")}`}
            />
            <FeatureItem
              icon={<Database className="h-4 w-4" />}
              title={t("settings:dataPersistence")}
              description={t("settings:dataPersistenceDesc")}
            />
            <FeatureItem
              icon={<Shield className="h-4 w-4" />}
              title={t("settings:ruleEngine")}
              description={`Pest DSL ${t("settings:parsing")}、${t("settings:realtimeEval")}`}
            />
            <FeatureItem
              icon={<Layers className="h-4 w-4" />}
              title={t("settings:pluginSystem")}
              description={t("settings:pluginSystemDesc")}
            />
          </div>
        </CardContent>
      </Card>

      {/* Project Links */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Book className="h-5 w-5 text-green-500" />
            {t("settings:projectInfo")}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm">
          <div className="flex items-center justify-between border-b pb-2">
            <span className="text-muted-foreground">{t("settings:version")}</span>
            <Badge variant="secondary">{systemInfo?.version || "v0.1.0"}</Badge>
          </div>
          <div className="flex items-center justify-between border-b pb-2">
            <span className="text-muted-foreground">{t("settings:license")}</span>
            <span>MIT</span>
          </div>
          <div className="flex items-center justify-between border-b pb-2">
            <span className="text-muted-foreground">{t("settings:architecture")}</span>
            <span>Workspace (Cargo)</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">{t("settings:repository")}</span>
            <a
              href="https://github.com/shenmingming/NeoTalk"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-500 hover:underline flex items-center gap-1"
            >
              <Globe className="h-3 w-3" />
              github.com/shenmingming/NeoTalk
            </a>
          </div>
        </CardContent>
      </Card>

      {/* Footer */}
      <div className="text-center text-sm text-muted-foreground">
        <p>© 2025 NeoTalk. {t("settings:builtWith")} ❤️ {t("settings:forIntelligence")}.</p>
      </div>
    </div>
  )
}

function FeatureItem({
  icon,
  title,
  description
}: {
  icon: React.ReactNode
  title: string
  description: string
}) {
  return (
    <div className="flex items-start gap-3 p-3 rounded-lg bg-muted/50">
      <div className="flex items-center justify-center w-8 h-8 rounded-md bg-background shrink-0">
        {icon}
      </div>
      <div>
        <div className="font-medium text-sm">{title}</div>
        <div className="text-xs text-muted-foreground">{description}</div>
      </div>
    </div>
  )
}
