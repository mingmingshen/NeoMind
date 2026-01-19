import { useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Sparkles, Clock, Home, Zap, Bell, ArrowRight } from 'lucide-react'
import { cn } from '@/lib/utils'

interface AutomationCreatorProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onCreate: (automation: {
    name: string
    description: string
    type: 'rule' | 'workflow'
    config: any
  }) => Promise<void>
}

// Predefined templates
const TEMPLATES = [
  {
    id: 'temp-schedule',
    name: '定时任务',
    icon: Clock,
    description: '按时间执行操作',
    color: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300',
    iconBg: 'bg-blue-500/10',
    iconColor: 'text-blue-500',
    examples: [
      '每天早上7点打开窗帘',
      '每天晚上10点关闭所有灯光',
      '每小时检查一次温度',
    ],
  },
  {
    id: 'temp-condition',
    name: '条件触发',
    icon: Zap,
    description: '满足条件时执行',
    color: 'bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300',
    iconBg: 'bg-amber-500/10',
    iconColor: 'text-amber-500',
    examples: [
      '温度超过30度时打开空调',
      '检测到有人时打开灯光',
      '湿度低于40%时开启加湿器',
    ],
  },
  {
    id: 'temp-scene',
    name: '场景模式',
    icon: Home,
    description: '一键控制多个设备',
    color: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300',
    iconBg: 'bg-green-500/10',
    iconColor: 'text-green-500',
    examples: [
      '回家模式：打开灯、调温度、播放音乐',
      '离家模式：关闭所有设备、开启安防',
      '睡眠模式：关闭灯光、降低音量',
    ],
  },
  {
    id: 'temp-alert',
    name: '告警通知',
    icon: Bell,
    description: '异常情况发送通知',
    color: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300',
    iconBg: 'bg-red-500/10',
    iconColor: 'text-red-500',
    examples: [
      '设备离线时发送通知',
      '温度异常时告警',
      '传感器故障时记录日志',
    ],
  },
]

export function AutomationCreator({
  open,
  onOpenChange,
  onCreate,
}: AutomationCreatorProps) {
  const [activeTab, setActiveTab] = useState<'natural' | 'template'>('natural')
  const [description, setDescription] = useState('')
  const [analyzing, setAnalyzing] = useState(false)
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null)

  // Generate automation from natural language
  const handleAnalyze = async () => {
    if (!description.trim()) return

    setAnalyzing(true)
    try {
      // Simulate AI analysis
      await new Promise((resolve) => setTimeout(resolve, 1000))

      // Parse the description to extract name
      const extractedName = description.split(/[，,。.时当]/)[0].trim()

      // Determine type based on keywords
      let type: 'rule' | 'workflow' = 'rule'
      if (description.includes('然后') || description.includes('之后') || description.includes('接着')) {
        type = 'workflow'
      }

      await onCreate({
        name: extractedName.slice(0, 20),
        description: description.slice(0, 100),
        type,
        config: { description },
      })

      // Reset
      setDescription('')
      onOpenChange(false)
    } catch (error) {
      console.error('Failed to create automation:', error)
    } finally {
      setAnalyzing(false)
    }
  }

  // Create from template
  const handleCreateFromTemplate = (templateId: string) => {
    setSelectedTemplate(templateId)
  }

  const getExamplePlaceholder = () => {
    const examples = [
      '当温度超过30度时，打开空调',
      '每天早上7点自动打开窗帘',
      '检测到有人时打开客厅灯光',
    ]
    return examples[Math.floor(Math.random() * examples.length)]
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-primary" />
            创建自动化
          </DialogTitle>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as any)} className="mt-4">
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="natural">
              <Sparkles className="h-4 w-4 mr-2" />
              智能描述
            </TabsTrigger>
            <TabsTrigger value="template">
              <Clock className="h-4 w-4 mr-2" />
              选择模板
            </TabsTrigger>
          </TabsList>

          {/* Natural Language Tab */}
          <TabsContent value="natural" className="space-y-4">
            <div className="text-center py-4">
              <p className="text-sm text-muted-foreground mb-4">
                用自然语言描述你想要实现的自动化，AI 会自动帮你生成
              </p>

              <div className="relative">
                <Textarea
                  placeholder={getExamplePlaceholder()}
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  className="min-h-[120px] resize-none text-base"
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                      handleAnalyze()
                    }
                  }}
                />
                <div className="absolute bottom-2 right-2 text-xs text-muted-foreground">
                  ⌘⏎ 提交
                </div>
              </div>

              {/* Quick Examples */}
              <div className="mt-4">
                <div className="text-xs text-muted-foreground mb-2">试试这些：</div>
                <div className="flex flex-wrap gap-2 justify-center">
                  {['温度超标开空调', '定时开关灯', '有人亮灯'].map((example) => (
                    <Button
                      key={example}
                      variant="outline"
                      size="sm"
                      className="h-7 text-xs"
                      onClick={() => setDescription(example)}
                    >
                      {example}
                    </Button>
                  ))}
                </div>
              </div>
            </div>

            <Button
              className="w-full"
              size="lg"
              onClick={handleAnalyze}
              disabled={!description.trim() || analyzing}
            >
              {analyzing ? (
                <>
                  <Sparkles className="h-4 w-4 mr-2 animate-pulse" />
                  分析中...
                </>
              ) : (
                <>
                  <Sparkles className="h-4 w-4 mr-2" />
                  生成自动化
                </>
              )}
            </Button>
          </TabsContent>

          {/* Template Tab */}
          <TabsContent value="template" className="space-y-4">
            <div className="grid grid-cols-2 gap-3">
              {TEMPLATES.map((template) => {
                const Icon = template.icon
                return (
                  <Card
                    key={template.id}
                    className={cn(
                      'cursor-pointer transition-all duration-200 hover:shadow-md',
                      selectedTemplate === template.id && 'ring-2 ring-primary'
                    )}
                    onClick={() => handleCreateFromTemplate(template.id)}
                  >
                    <CardContent className="p-4">
                      <div className="flex items-start gap-3">
                        <div className={cn('p-2 rounded-lg', template.iconBg)}>
                          <Icon className={cn('h-5 w-5', template.iconColor)} />
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <h4 className="font-medium text-sm">{template.name}</h4>
                            <Badge variant="outline" className={cn('text-xs', template.color)}>
                              {template.examples.length}
                            </Badge>
                          </div>
                          <p className="text-xs text-muted-foreground mt-0.5">
                            {template.description}
                          </p>
                          <div className="mt-2 space-y-1">
                            {template.examples.slice(0, 2).map((example, i) => (
                              <div
                                key={i}
                                className="text-xs text-muted-foreground truncate bg-muted/50 px-2 py-1 rounded"
                              >
                                {example}
                              </div>
                            ))}
                          </div>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                )
              })}
            </div>

            {/* Example Descriptions for Selected Template */}
            {selectedTemplate && (
              <Card className="mt-4">
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm font-medium">选择一个示例开始：</CardTitle>
                </CardHeader>
                <CardContent className="pt-0">
                  <div className="space-y-2">
                    {TEMPLATES.find((t) => t.id === selectedTemplate)?.examples.map(
                      (example, i) => (
                        <button
                          key={i}
                          className="w-full text-left p-3 bg-background rounded-md hover:bg-muted transition-all duration-200 text-sm border border-border hover:border-primary/50"
                          onClick={() => {
                            setDescription(example)
                            setActiveTab('natural')
                          }}
                        >
                          <div className="flex items-center justify-between">
                            <span>{example}</span>
                            <ArrowRight className="h-4 w-4 text-muted-foreground" />
                          </div>
                        </button>
                      )
                    )}
                  </div>
                </CardContent>
              </Card>
            )}
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
