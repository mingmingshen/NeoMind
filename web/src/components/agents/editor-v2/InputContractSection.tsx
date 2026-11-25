/**
 * InputContract requirements list (read-only summary + key editor for M3 scaffold).
 */
import { useTranslation } from 'react-i18next'
import { Database } from '@/design-system/icons'
import { FormField } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { cardBase } from '@/design-system/tokens/size'
import { cn } from '@/lib/utils'
import type { InputContract, InputRequirement } from '@/types'

export interface InputContractSectionProps {
  value: InputContract
  onChange: (next: InputContract) => void
  className?: string
}

function sourceLabel(req: InputRequirement): string {
  const s = req.source
  switch (s.kind) {
    case 'extension_instance':
      return `extension:${s.id}${req.operation ? `:${req.operation}` : ''}`
    case 'device_metric':
      return `device:${s.device_id}:${s.field}`
    case 'whole_device':
      return `device:${s.device_id}`
    case 'upstream_ai':
      return `ai:${s.agent_id}:${s.field}`
    case 'caller_payload':
      return 'caller'
  }
}

export function InputContractSection({ value, onChange, className }: InputContractSectionProps) {
  const { t } = useTranslation('agents')
  const requirements = value.requirements ?? []

  const updateKey = (index: number, key: string) => {
    const next = requirements.map((r, i) => (i === index ? { ...r, key } : r))
    onChange({ requirements: next })
  }

  return (
    <section className={cn(cardBase, 'p-4 space-y-4', className)} data-section="input">
      <header className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Database className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium text-foreground">
            {t('editorV2.input.title')}
          </h3>
        </div>
        <span className="text-xs text-muted-foreground">
          {t('editorV2.input.count', { count: requirements.length })}
        </span>
      </header>

      {requirements.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('editorV2.input.empty')}</p>
      ) : (
        <ul className="space-y-3">
          {requirements.map((req, index) => (
            <li
              key={`${req.key}-${index}`}
              className="rounded-md border border-border p-3 space-y-2"
            >
              <FormField label={t('editorV2.input.key')}>
                <Input
                  value={req.key}
                  onChange={(e) => updateKey(index, e.target.value)}
                  className="font-mono text-sm"
                />
              </FormField>
              <p className="text-xs text-muted-foreground font-mono">{sourceLabel(req)}</p>
              {req.auth_scope && (
                <p className="text-xs text-muted-foreground">
                  {t('editorV2.input.auth')}: {req.auth_scope}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}

      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled
        title={t('editorV2.input.addSoon')}
      >
        {t('editorV2.input.add')}
      </Button>
    </section>
  )
}
