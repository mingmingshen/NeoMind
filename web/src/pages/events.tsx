import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Bell,
  Terminal,
  Activity,
  Sparkles,
  Check,
} from 'lucide-react'
import { PageLayout } from '@/components/layout/PageLayout'
import { PageTabs, PageTabsContent } from '@/components/shared'
import { AlertsTab } from '@/components/automation/AlertsTab'
import { CommandsTab } from '@/components/automation/CommandsTab'
import { api } from '@/lib/api'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { EmptyStateInline, ActionBar } from '@/components/shared'
import { useApiData } from '@/hooks/useApiData'
import { formatTimestamp } from '@/lib/utils/format'
import type { Event as SystemEvent } from '@/types'

type ActiveTab = 'alerts' | 'commands' | 'events'

const fetchEvents = async (): Promise<SystemEvent[]> => {
  const response = await api.getEvents({ limit: 100 }) as unknown as { events: SystemEvent[]; count: number }
  return response.events || []
}

export function EventsPage() {
  const { t } = useTranslation(['common', 'events', 'alerts', 'commands'])
  const [activeTab, setActiveTab] = useState<ActiveTab>('alerts')

  const tabs = [
    {
      value: 'alerts' as ActiveTab,
      label: t('events:alerts', { defaultValue: 'Alerts' }),
      icon: <Bell className="h-4 w-4" />,
    },
    {
      value: 'commands' as ActiveTab,
      label: t('events:commands', { defaultValue: 'Commands' }),
      icon: <Terminal className="h-4 w-4" />,
    },
    {
      value: 'events' as ActiveTab,
      label: t('events:systemEvents', { defaultValue: 'Events' }),
      icon: <Activity className="h-4 w-4" />,
    },
  ]

  const { data: events, loading, refetch } = useApiData(
    () => fetchEvents(),
    { deps: [] }
  )

  const getEventTypeBadge = (eventType: string) => {
    const type = eventType.toLowerCase()
    let variant: 'default' | 'secondary' | 'destructive' | 'outline' = 'secondary'
    let color = ''

    if (type.includes('device') || type.includes('online') || type.includes('offline')) {
      variant = 'outline'
      color = 'text-blue-600'
    } else if (type.includes('rule') || type.includes('trigger')) {
      variant = 'default'
      color = 'text-yellow-600'
    } else if (type.includes('workflow')) {
      variant = 'default'
      color = 'text-purple-600'
    } else if (type.includes('alert')) {
      variant = 'destructive'
      color = 'text-red-600'
    } else if (type.includes('automation') || type.includes('llm')) {
      variant = 'default'
      color = 'text-green-600'
    }

    return (
      <Badge variant={variant} className={color}>
        {eventType}
      </Badge>
    )
  }

  const getEventIcon = (eventType: string) => {
    const type = eventType.toLowerCase()

    if (type.includes('device') || type.includes('online') || type.includes('offline')) {
      return <Activity className="h-4 w-4 text-blue-500" />
    }
    if (type.includes('rule') || type.includes('trigger')) {
      return <Sparkles className="h-4 w-4 text-yellow-500" />
    }
    if (type.includes('workflow')) {
      return <Terminal className="h-4 w-4 text-purple-500" />
    }
    if (type.includes('alert')) {
      return <Bell className="h-4 w-4 text-red-500" />
    }
    if (type.includes('automation') || type.includes('llm')) {
      return <Check className="h-4 w-4 text-green-500" />
    }

    return <Activity className="h-4 w-4" />
  }

  return (
    <PageLayout>
      <PageTabs
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(v) => setActiveTab(v as ActiveTab)}
      >
        <PageTabsContent value="alerts" activeTab={activeTab}>
          <AlertsTab />
        </PageTabsContent>

        <PageTabsContent value="commands" activeTab={activeTab}>
          <CommandsTab />
        </PageTabsContent>

        <PageTabsContent value="events" activeTab={activeTab}>
          <div className="space-y-4">
            {/* Header */}
            <ActionBar
              title={t('events:systemEvents', { defaultValue: 'System Events' })}
              titleIcon={<Activity className="h-5 w-5" />}
              description={t('events:systemEventsDesc', {
                defaultValue: 'Real-time system events and activity log',
              })}
              onRefresh={refetch}
            />

            {/* Events Table */}
            <Card>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t('events:type', { defaultValue: 'Type' })}</TableHead>
                    <TableHead>{t('events:source', { defaultValue: 'Source' })}</TableHead>
                    <TableHead>{t('events:data', { defaultValue: 'Data' })}</TableHead>
                    <TableHead>{t('events:timestamp', { defaultValue: 'Timestamp' })}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {loading ? (
                    <EmptyStateInline title={t('common:loading')} colSpan={4} />
                  ) : !events || events.length === 0 ? (
                    <EmptyStateInline
                      title={t('events:noEvents', { defaultValue: 'No events found' })}
                      colSpan={4}
                    />
                  ) : (
                    events.map((event) => (
                      <TableRow key={event.id}>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            {getEventIcon(event.event_type)}
                            {getEventTypeBadge(event.event_type)}
                          </div>
                        </TableCell>
                        <TableCell>
                          {event.source && (
                            <span className="font-mono text-xs">{event.source}</span>
                          )}
                        </TableCell>
                        <TableCell>
                          <div className="max-w-md overflow-hidden">
                            <pre className="text-xs text-muted-foreground truncate">
                              {JSON.stringify(event.data, null, 2)}
                            </pre>
                          </div>
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">
                          {formatTimestamp(event.timestamp)}
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </Card>
          </div>
        </PageTabsContent>
      </PageTabs>
    </PageLayout>
  )
}
