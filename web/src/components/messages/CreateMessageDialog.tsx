// CreateMessageDialog Component
// Dialog for creating new messages/notifications
// Uses UnifiedFormDialog for consistent styling across mobile and desktop

import { useState, useCallback, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { MessageSquare, Bell, Database, Info, AlertTriangle } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { FormField } from '@/components/ui/field'
import { FormSection, FormSectionGroup } from '@/components/ui/form-section'
import { UnifiedFormDialog } from '@/components/dialog/UnifiedFormDialog'
import type { CreateMessageRequest, MessageSeverity, MessageCategory, MessageType } from '@/types'
import { useFormSubmit } from '@/hooks/useErrorHandler'

// Default payload example for Data Push messages
const DEFAULT_PAYLOAD_EXAMPLE = JSON.stringify({
  event: "sensor_data",
  device_id: "device_001",
  timestamp: new Date().toISOString(),
  data: {
    temperature: 25.5,
    humidity: 60,
    status: "normal"
  },
  metadata: {
    source: "iot_gateway",
    version: "1.0"
  }
}, null, 2)

interface CreateMessageDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onCreate: (req: CreateMessageRequest) => Promise<void>
}

export function CreateMessageDialog({ open, onOpenChange, onCreate }: CreateMessageDialogProps) {
  const { t } = useTranslation()

  const [messageType, setMessageType] = useState<MessageType>('notification')
  const [category, setCategory] = useState<MessageCategory>('alert')
  const [severity, setSeverity] = useState<MessageSeverity>('info')
  const [title, setTitle] = useState('')
  const [message, setMessage] = useState('')
  const [source, setSource] = useState('')
  const [sourceType, setSourceType] = useState('')
  const [sourceId, setSourceId] = useState('')
  const [tags, setTags] = useState('')
  const [payload, setPayload] = useState('')

  // Auto-fill default payload example when switching to data_push type
  useEffect(() => {
    if (messageType === 'data_push' && !payload.trim()) {
      setPayload(DEFAULT_PAYLOAD_EXAMPLE)
    }
  }, [messageType, payload])

  // Validation state
  const [titleError, setTitleError] = useState<string | null>(null)
  const [messageError, setMessageError] = useState<string | null>(null)
  const [payloadError, setPayloadError] = useState<string | null>(null)

  const { isSubmitting, handleSubmit: wrapSubmit } = useFormSubmit({
    onSuccess: () => {
      // Reset form
      setMessageType('notification')
      setTitle('')
      setMessage('')
      setSource('')
      setSourceType('')
      setSourceId('')
      setTags('')
      setPayload('')
      setSeverity('info')
      setCategory('alert')
      setTitleError(null)
      setMessageError(null)
      setPayloadError(null)
      onOpenChange(false)
    },
    errorOperation: 'Create message',
  })

  const validateForm = useCallback(() => {
    let isValid = true

    if (!title.trim()) {
      setTitleError(t('messages.formTitle.required', { defaultValue: 'Title is required' }))
      isValid = false
    } else {
      setTitleError(null)
    }

    if (!message.trim()) {
      setMessageError(t('messages.content.required', { defaultValue: 'Message is required' }))
      isValid = false
    } else {
      setMessageError(null)
    }

    // Validate payload JSON if message type is data_push
    if (messageType === 'data_push' && payload.trim()) {
      try {
        JSON.parse(payload)
        setPayloadError(null)
      } catch {
        setPayloadError(t('messages.payload.invalidJson', { defaultValue: 'Invalid JSON format' }))
        isValid = false
      }
    } else {
      setPayloadError(null)
    }

    return isValid
  }, [title, message, messageType, payload, t])

  const handleFormSubmit = useCallback(async () => {
    if (!validateForm()) return

    await wrapSubmit(async () => {
      const request: CreateMessageRequest = {
        category,
        severity,
        title: title.trim(),
        message: message.trim(),
        source: source || undefined,
        source_type: sourceType || undefined,
        source_id: sourceId || undefined,
        tags: tags ? tags.split(',').map(t => t.trim()).filter(Boolean) : undefined,
        message_type: messageType,
        payload: messageType === 'data_push' && payload.trim() ? JSON.parse(payload) : undefined,
      }
      await onCreate(request)
    })()
  }, [validateForm, wrapSubmit, onCreate, category, severity, title, message, source, sourceType, sourceId, tags, messageType, payload])

  return (
    <UnifiedFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('messages.createTitle')}
      description={t('messages.createDescription')}
      icon={<MessageSquare className="h-5 w-5" />}
      width="md"
      onSubmit={handleFormSubmit}
      isSubmitting={isSubmitting}
      submitLabel={t('common:create')}
      loading={false}
    >
      <FormSectionGroup>
        {/* Basic Settings Section */}
        <FormSection
          title={t('messages.basicSettings', { defaultValue: 'Basic Settings' })}
          description={t('messages.basicSettingsDesc', { defaultValue: 'Configure message type, category and severity' })}
        >
          <div className="space-y-4">
            {/* Message Type - Full width at top */}
            <FormField
              label={t('messages.type.label')}
              helpText={messageType === 'notification'
                ? t('messages.type.notificationHint', 'Standard notification message for alerts and updates')
                : t('messages.type.dataPushHint', 'Structured data for system integration and webhook delivery')
              }
            >
              <Select value={messageType} onValueChange={(v) => setMessageType(v as MessageType)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="notification">
                    <div className="flex items-center gap-2">
                      <Bell className="h-4 w-4" />
                      {t('messages.type.notification')}
                    </div>
                  </SelectItem>
                  <SelectItem value="data_push">
                    <div className="flex items-center gap-2">
                      <Database className="h-4 w-4" />
                      {t('messages.type.data_push')}
                    </div>
                  </SelectItem>
                </SelectContent>
              </Select>
            </FormField>

            {/* Data Push Info */}
            {messageType === 'data_push' && (
              <div className="flex items-start gap-2 p-3 rounded-lg bg-accent-purple-light border border-accent-purple-light">
                <Info className="h-4 w-4 text-accent-purple shrink-0 mt-0.5" />
                <p className="text-xs text-accent-purple">
                  {t('messages.type.dataPushInfo', 'Data Push messages are designed for system integration. The payload will be delivered to configured webhook channels. Make sure to include structured data in the Payload section.')}
                </p>
              </div>
            )}

            {/* Category and Severity on same row */}
            <div className="grid grid-cols-2 gap-4">
              <FormField label={t('messages.category.label')}>
                <Select value={category} onValueChange={(v) => setCategory(v as MessageCategory)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="alert">{t('messages.category.alert')}</SelectItem>
                    <SelectItem value="system">{t('messages.category.system')}</SelectItem>
                    <SelectItem value="business">{t('messages.category.business')}</SelectItem>
                  </SelectContent>
                </Select>
              </FormField>

              <FormField label={t('messages.severity.label')}>
                <Select value={severity} onValueChange={(v) => setSeverity(v as MessageSeverity)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="info">{t('messages.severity.info')}</SelectItem>
                    <SelectItem value="warning">{t('messages.severity.warning')}</SelectItem>
                    <SelectItem value="critical">{t('messages.severity.critical')}</SelectItem>
                    <SelectItem value="emergency">{t('messages.severity.emergency')}</SelectItem>
                  </SelectContent>
                </Select>
              </FormField>
            </div>
          </div>
        </FormSection>

        {/* Content Section */}
        <FormSection
          title={t('messages.contentSection', { defaultValue: 'Content' })}
          description={t('messages.contentSectionDesc', { defaultValue: 'Enter message title and body' })}
        >
          <div className="space-y-4">
            <FormField
              label={t('messages.formTitle.label')}
              required
              error={titleError || undefined}
            >
              <Input
                value={title}
                onChange={(e) => {
                  setTitle(e.target.value)
                  if (titleError) setTitleError(null)
                }}
                placeholder={t('messages.formTitle.placeholder')}
                autoFocus
              />
            </FormField>

            <FormField
              label={t('messages.content.label')}
              required
              error={messageError || undefined}
            >
              <Textarea
                value={message}
                onChange={(e) => {
                  setMessage(e.target.value)
                  if (messageError) setMessageError(null)
                }}
                placeholder={t('messages.content.placeholder')}
                rows={3}
              />
            </FormField>
          </div>
        </FormSection>

        {/* Payload Section - Only for DataPush */}
        {messageType === 'data_push' && (
          <FormSection
            title={t('messages.payload.section', { defaultValue: 'Payload Data' })}
            description={t('messages.payload.sectionDesc', { defaultValue: 'Structured data for Data Push messages (JSON format)' })}
          >
            <FormField
              label={t('messages.payload.label', { defaultValue: 'Payload (JSON)' })}
              error={payloadError || undefined}
            >
              <Textarea
                value={payload}
                onChange={(e) => {
                  setPayload(e.target.value)
                  if (payloadError) setPayloadError(null)
                }}
                placeholder='{"key": "value", "data": {...}}'
                rows={5}
                className="font-mono text-sm"
              />
            </FormField>
          </FormSection>
        )}

        {/* Source Section */}
        <FormSection
          title={t('messages.sourceSection', { defaultValue: 'Source (Optional)' })}
          description={t('messages.sourceSectionDesc', { defaultValue: 'Identify the source of this message' })}
          collapsible
          defaultExpanded={false}
        >
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <FormField
              label={t('messages.sourceLabel')}
            >
              <Input
                value={source}
                onChange={(e) => setSource(e.target.value)}
                placeholder={t('messages.sourcePlaceholder')}
              />
            </FormField>

            <FormField
              label={t('messages.sourceType.label')}
            >
              <Input
                value={sourceType}
                onChange={(e) => setSourceType(e.target.value)}
                placeholder={t('messages.sourceType.placeholder')}
              />
            </FormField>

            <FormField
              label={t('messages.sourceId.label', { defaultValue: 'Source ID' })}
              helpText={t('messages.sourceId.hint', { defaultValue: 'Unique identifier for filtering (e.g., device:001)' })}
            >
              <Input
                value={sourceId}
                onChange={(e) => setSourceId(e.target.value)}
                placeholder="device:001"
              />
            </FormField>

            <FormField
              label={t('messages.tags.label')}
              helpText={t('messages.tags.hint')}
            >
              <Input
                value={tags}
                onChange={(e) => setTags(e.target.value)}
                placeholder="tag1, tag2, tag3"
              />
            </FormField>
          </div>
        </FormSection>
      </FormSectionGroup>
    </UnifiedFormDialog>
  )
}
