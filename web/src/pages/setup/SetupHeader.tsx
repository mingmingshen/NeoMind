/**
 * Shared header for setup pages with language switcher and optional back button
 */
import { useTranslation } from 'react-i18next'
import { ArrowLeft, Languages } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

const languages = [
  { code: 'en', name: 'English' },
  { code: 'zh', name: '简体中文' },
]

interface SetupHeaderProps {
  onBack?: () => void
  stepLabel?: string
}

export function SetupHeader({ onBack, stepLabel }: SetupHeaderProps) {
  const { t, i18n } = useTranslation(['common', 'setup'])

  return (
    <header className="relative z-10 backdrop-blur-sm">
      <div className="flex items-center justify-between px-4 h-14 sm:px-6 sm:h-16">
        <div className="flex items-center gap-3">
          {onBack && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onBack}
              className="gap-1.5"
            >
              <ArrowLeft className="h-4 w-4" />
              {t('setup:back')}
            </Button>
          )}
          {stepLabel && (
            <span className="text-sm text-muted-foreground">{stepLabel}</span>
          )}
        </div>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm" className="gap-1.5">
              <Languages className="h-4 w-4" />
              {languages.find(l => l.code === i18n.language)?.name || 'Language'}
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
    </header>
  )
}
