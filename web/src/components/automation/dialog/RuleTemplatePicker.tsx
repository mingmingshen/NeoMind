import { useTranslation } from 'react-i18next'
import { UnifiedFormDialog } from '@/components/dialog/UnifiedFormDialog'
import { Card, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { RULE_TEMPLATES } from '../ruleTemplates'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSelectTemplate: (templateId: string) => void
}

export function RuleTemplatePicker({ open, onOpenChange, onSelectTemplate }: Props) {
  const { t } = useTranslation()
  return (
    <UnifiedFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('rules.templates.title')}
      description={t('rules.templates.subtitle')}
      hideFooter
      cancelLabel={t('common:cancel')}
    >
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {RULE_TEMPLATES.map((tpl) => {
          const Icon = tpl.icon
          return (
            <Card
              key={tpl.id}
              className="cursor-pointer hover:bg-muted-30 transition-colors"
              onClick={() => onSelectTemplate(tpl.id)}
            >
              <CardHeader>
                <div className="flex items-center gap-2">
                  <Icon className="h-5 w-5 text-primary" />
                  <CardTitle>{t(tpl.labelKey)}</CardTitle>
                </div>
                <CardDescription>{t(tpl.descriptionKey)}</CardDescription>
              </CardHeader>
            </Card>
          )
        })}
      </div>
    </UnifiedFormDialog>
  )
}
