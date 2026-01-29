/**
 * ComponentConfigDialog Component
 *
 * Modern unified dialog for configuring dashboard components.
 * Layout: Two-column (Preview + Config) with modern styling.
 * Fully responsive with touch-friendly controls.
 */

import { useMemo, useState, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Settings,
  CheckCircle2,
  Eye,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Field } from '@/components/ui/field'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import { ConfigRenderer } from './ConfigRenderer'
import { ComponentPreview } from './ComponentPreview'
import { UnifiedDataSourceConfig } from './UnifiedDataSourceConfig'
import { DataTransformConfig } from './DataTransformConfig'
import type { ComponentConfigSchema } from './ComponentConfigBuilder'
import type { DataSource, DataSourceOrList } from '@/types/dashboard'
import { normalizeDataSource } from '@/types/dashboard'
import { cn } from '@/lib/utils'

export interface ComponentConfigDialogProps {
  open: boolean
  onClose: () => void
  onSave: () => void
  title: string
  onTitleChange: (title: string) => void
  configSchema: ComponentConfigSchema | null
  componentType: string
  // Preview props
  previewDataSource?: DataSource
  previewConfig?: Record<string, unknown>
  // Show title in display section instead of separate input
  showTitleInDisplay?: boolean
}

export function ComponentConfigDialog({
  open,
  onClose,
  onSave,
  title,
  onTitleChange,
  configSchema,
  componentType,
  previewDataSource,
  previewConfig = {},
  showTitleInDisplay = true,
}: ComponentConfigDialogProps) {
  const { t } = useTranslation('dashboardComponents')
  // Extract config schema sections first (before using them in useMemo)
  const dataSourceSections = configSchema?.dataSourceSections ?? []
  const styleSections = configSchema?.styleSections ?? []
  const displaySections = configSchema?.displaySections ?? []
  // Fallback for legacy configs
  const allSections = configSchema?.sections ?? []

  const hasDataSource = dataSourceSections.length > 0 || allSections.some(s => s.type === 'data-source')
  const hasStyleConfig = styleSections.length > 0
  // Display config exists when there are display sections OR when title is shown in display tab
  const hasDisplayConfig = showTitleInDisplay || displaySections.length > 0 || allSections.some(s => s.type !== 'data-source')
  const hasAnyConfig = hasDataSource || hasStyleConfig || hasDisplayConfig || allSections.length > 0

  // Mobile: Inner tab state for data source / transform switching
  const [mobileDataSourceTab, setMobileDataSourceTab] = useState<'datasource' | 'transform'>('datasource')

  // Desktop: Right column tab state for data source / transform switching
  const [rightDataSourceTab, setRightDataSourceTab] = useState<'datasource' | 'transform'>('datasource')

  // Config tab state - default to 'display' if no style config, otherwise 'style'
  const [configTabValue, setConfigTabValue] = useState<'style' | 'display'>('display')

  // Reset tabs when dialog opens
  useEffect(() => {
    if (open) {
      setMobileDataSourceTab('datasource')
      setRightDataSourceTab('datasource')
      // Set default config tab based on available sections
      setConfigTabValue(hasStyleConfig ? 'style' : 'display')
    }
  }, [open, hasStyleConfig])

  // Extract data source section props
  const dataSourceSection = [...dataSourceSections, ...allSections].find(s => s.type === 'data-source')
  const dataSourceProps = dataSourceSection?.type === 'data-source' ? dataSourceSection.props : null
  const multiple = dataSourceProps?.multiple ?? false
  const maxSources = dataSourceProps?.maxSources

  // Check if data source is configured
  const normalizedSources = previewDataSource ? normalizeDataSource(previewDataSource) : []
  const hasConfiguredDataSource = normalizedSources.length > 0

  // Check if any data source is device-info (which doesn't have historical data)
  const hasDeviceInfoOnly = useMemo(() => {
    return normalizedSources.length > 0 && normalizedSources.every((ds: DataSource) => ds.type === 'device-info')
  }, [normalizedSources])

  // Components that support time-series data transformation
  const supportsDataTransform = useMemo(() => {
    const transformCapableTypes: string[] = [
      // Charts
      'line-chart',
      'area-chart',
      'bar-chart',
      'pie-chart',
      // Indicators that have time-series data
      'value-card',
      'sparkline',
      'progress-bar',
      // Note: led-indicator doesn't need transform (single state, not time-series)
    ]
    return transformCapableTypes.includes(componentType)
  }, [componentType])

  // Map component type to chart type for DataTransformConfig
  const getChartTypeForTransform = (type: string): 'pie' | 'bar' | 'line' | 'area' | 'card' | 'sparkline' | 'progress' => {
    if (type.endsWith('-chart')) {
      return type.replace(/-chart$/, '') as 'pie' | 'bar' | 'line' | 'area'
    }
    switch (type) {
      case 'value-card':
        return 'card'
      case 'sparkline':
        return 'sparkline'
      case 'progress-bar':
        return 'progress'
      default:
        return 'bar'
    }
  }

  // Show data transform config only when:
  // 1. Data source is configured
  // 2. Component type supports data transformation
  // 3. Data source is NOT device-info only (no historical data)
  const shouldShowDataTransform = hasConfiguredDataSource && supportsDataTransform && !hasDeviceInfoOnly

  const handleDataSourceChange = (dataSource: DataSourceOrList | DataSource | undefined) => {
    dataSourceProps?.onChange(dataSource as any)
  }

  // Create a stable key for ComponentPreview to force re-render when dataSource selection changes
  // The key should NOT change when transform settings change, only when selection changes
  const [previewKey, setPreviewKey] = useState<string>('preview-no-ds')
  const coreIdentifier = useMemo(() => {
    if (!previewDataSource) return 'preview-no-ds'
    const sources = normalizeDataSource(previewDataSource)
    return sources.map(s => `${s.type}:${s.deviceId || ''}:${s.metricId || s.property || s.infoProperty || ''}:${s.command || ''}`).join('|')
  }, [previewDataSource])

  // Only update the key state when core identifier actually changes
  useEffect(() => {
    setPreviewKey(coreIdentifier)
  }, [coreIdentifier])

  // Handle data transform configuration changes
  const handleDataTransformChange = (updates: Partial<DataSource>) => {
    if (!previewDataSource) return

    // Handle both single DataSource and DataSource[] (multiple sources)
    const sources = normalizeDataSource(previewDataSource)

    // Apply transform updates to all sources
    const updatedSources = sources.map(source => ({
      ...source,
      ...updates,
    }))

    // Return in the same format as input (array or single)
    const result = Array.isArray(previewDataSource) ? updatedSources : updatedSources[0]
    dataSourceProps?.onChange(result as any)
  }

  // Update style sections to remove data-source section from legacy configs
  const filteredStyleSections = styleSections.length > 0
    ? styleSections
    : allSections.filter(s => s.type !== 'data-source')

  // Live preview config - combines initial config with current title
  const livePreviewConfig = useMemo(() => ({
    ...previewConfig,
    title,  // Title changes in real-time
  }), [previewConfig, title])

  // Live preview data source
  const livePreviewDataSource = useMemo(() => {
    // Use previewDataSource from props (which comes from componentConfig and updates live)
    // This ensures the preview updates as soon as config changes
    return previewDataSource
  }, [previewDataSource])

  // Create a title section for display
  const titleSection = useMemo(() => showTitleInDisplay ? [{
    type: 'custom' as const,
    render: () => (
      <Field>
        <Label htmlFor="component-title-display">{t('componentConfig.displayTitle')}</Label>
        <Input
          id="component-title-display"
          value={title}
          onChange={(e) => onTitleChange(e.target.value)}
          placeholder={t('componentConfig.titlePlaceholder')}
          className="h-10"
        />
      </Field>
    ),
  }] : [], [showTitleInDisplay, title, onTitleChange, t])

  // Enhanced display sections with title
  const enhancedDisplaySections = useMemo(() => {
    if (showTitleInDisplay && displaySections.length > 0) {
      return [...titleSection, ...displaySections]
    }
    return displaySections
  }, [showTitleInDisplay, titleSection, displaySections])

  // Use enhanced display sections when showTitleInDisplay is true, otherwise use original
  // This ensures title is prepended to display sections when needed
  const finalDisplaySections = showTitleInDisplay ? enhancedDisplaySections : displaySections

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="
        max-w-[95vw] w-[1100px]
        p-0 gap-0 h-[850px] overflow-hidden flex flex-col
        [&>[data-radix-dialog-close]]:right-4 [&>[data-radix-dialog-close]]:top-5
        rounded-2xl
      ">
        {/* Header */}
        <DialogHeader className="px-6 py-4 border-b shrink-0 bg-gradient-to-r from-primary/5 via-background to-background">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center border border-primary/20">
                <Settings className="h-5 w-5 text-primary" />
              </div>
              <div>
                <DialogTitle className="text-base font-semibold p-0 h-auto">
                  {t('componentConfig.editComponent')}
                </DialogTitle>
                <p className="text-xs text-muted-foreground mt-0.5 font-medium">
                  {componentType.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}
                </p>
              </div>
            </div>
          </div>
        </DialogHeader>

        {/* Content Area */}
        {/* Large screens (>= 1024px): Two-column layout */}
        {/* Small screens (< 1024px): Tab-based layout */}
        <div className="flex-1 flex flex-col lg:flex-row overflow-hidden">
          {/* Small screen: Tab-based layout */}
          <div className="flex-1 flex flex-col lg:hidden overflow-hidden">
            <Tabs defaultValue="preview" className="flex-1 flex flex-col">
              <TabsList className="grid w-full grid-cols-2 h-11 bg-muted/50 p-1.5 rounded-xl mx-4 mt-4">
                <TabsTrigger value="preview" className="gap-2 rounded-lg data-[state=active]:bg-background data-[state=active]:shadow-sm">
                  <Eye className="h-4 w-4" />
                  <span className="font-medium">{t('componentConfig.preview')}</span>
                </TabsTrigger>
                <TabsTrigger value="config" className="gap-2 rounded-lg data-[state=active]:bg-background data-[state=active]:shadow-sm">
                  <Settings className="h-4 w-4" />
                  <span className="font-medium">{t('componentConfig.config')}</span>
                </TabsTrigger>
              </TabsList>

              {/* Preview Tab */}
              <TabsContent value="preview" className="flex-1 min-h-0 overflow-y-auto px-4 pb-4 mt-2">
                <div className="rounded-xl border bg-muted/20 p-4">
                  <ComponentPreview
                    key={previewKey}
                    componentType={componentType}
                    config={livePreviewConfig}
                    dataSource={livePreviewDataSource}
                    title={title}
                    showHeader={true}
                  />
                </div>
              </TabsContent>

              {/* Config Tab */}
              <TabsContent value="config" className="flex-1 min-h-0 overflow-y-auto flex flex-col">
                {/* Data Source + Transform Section (Mobile) */}
                {hasDataSource && (
                  <div className="rounded-xl border bg-card overflow-hidden mx-4 mt-2 shrink-0">
                    {/* Tab List - Manual implementation for mobile to avoid nested Tabs */}
                    {shouldShowDataTransform ? (
                      <>
                        <div className="flex items-center gap-2 px-4 py-3 border-b bg-muted/30">
                          <div className="flex gap-1">
                            <button
                              onClick={() => setMobileDataSourceTab('datasource')}
                              className={`px-3 h-8 text-sm rounded-none transition-colors border-b-2 ${
                                mobileDataSourceTab === 'datasource'
                                  ? 'border-primary text-foreground'
                                  : 'border-transparent text-muted-foreground hover:text-foreground'
                              }`}
                            >
                              {t('componentConfig.dataSource')}
                            </button>
                            <button
                              onClick={() => setMobileDataSourceTab('transform')}
                              className={`px-3 h-8 text-sm rounded-none transition-colors border-b-2 ${
                                mobileDataSourceTab === 'transform'
                                  ? 'border-primary text-foreground'
                                  : 'border-transparent text-muted-foreground hover:text-foreground'
                              }`}
                            >
                              {t('componentConfig.transform')}
                            </button>
                          </div>
                          {hasConfiguredDataSource && (
                            <CheckCircle2 className="h-4 w-4 text-green-500 ml-auto" />
                          )}
                        </div>

                        {/* Data Source Tab Content */}
                        {mobileDataSourceTab === 'datasource' && (
                          <div className="p-4">
                            <UnifiedDataSourceConfig
                              value={previewDataSource}
                              onChange={handleDataSourceChange}
                              allowedTypes={dataSourceProps?.allowedTypes}
                              multiple={multiple}
                              maxSources={maxSources}
                            />
                          </div>
                        )}

                        {/* Transform Tab Content */}
                        {mobileDataSourceTab === 'transform' && (
                          <div className="p-4">
                            <DataTransformConfig
                              dataSource={previewDataSource}
                              onChange={handleDataTransformChange}
                              chartType={getChartTypeForTransform(componentType)}
                            />
                          </div>
                        )}
                      </>
                    ) : (
                      <>
                        <div className="flex items-center gap-2 px-4 py-3 border-b bg-muted/30">
                          <Settings className="h-4 w-4 text-primary" />
                          <span className="text-sm font-semibold">{t('componentConfig.dataSourceConfig')}</span>
                          {hasConfiguredDataSource && (
                            <CheckCircle2 className="h-4 w-4 text-green-500 ml-auto" />
                          )}
                        </div>
                        <div className="p-4">
                          <UnifiedDataSourceConfig
                            value={previewDataSource}
                            onChange={handleDataSourceChange}
                            allowedTypes={dataSourceProps?.allowedTypes}
                            multiple={multiple}
                            maxSources={maxSources}
                          />
                        </div>
                      </>
                    )}
                  </div>
                )}

                {/* Config Tabs - Style, Display */}
                {(hasStyleConfig || hasDisplayConfig) && (
                  <Tabs value={configTabValue} onValueChange={(v) => setConfigTabValue(v as 'style' | 'display')} className="flex-1 flex flex-col min-h-0 mx-4 mt-3 overflow-hidden">
                    <TabsList className="w-full justify-start bg-muted/50 p-1 rounded-xl h-11 shrink-0">
                      {hasStyleConfig && (
                        <TabsTrigger value="style" className="gap-2 data-[state=active]:bg-background data-[state=active]:shadow-sm rounded-lg">
                          {t('componentConfig.style')}
                        </TabsTrigger>
                      )}
                      {hasDisplayConfig && (
                        <TabsTrigger value="display" className="gap-2 data-[state=active]:bg-background data-[state=active]:shadow-sm rounded-lg">
                          {t('componentConfig.display')}
                        </TabsTrigger>
                      )}
                    </TabsList>

                    {hasStyleConfig && (
                      <TabsContent value="style" className="flex-1 min-h-0 overflow-y-auto mt-3 data-[state=active]:flex data-[state=active]:flex-col">
                        <ConfigRenderer sections={filteredStyleSections} />
                      </TabsContent>
                    )}

                    {hasDisplayConfig && (
                      <TabsContent value="display" className="flex-1 min-h-0 overflow-y-auto mt-3 data-[state=active]:flex data-[state=active]:flex-col">
                        <ConfigRenderer sections={finalDisplaySections} />
                      </TabsContent>
                    )}
                  </Tabs>
                )}

                {/* Legacy sections fallback */}
                {!hasDataSource && !hasStyleConfig && !displaySections.length && allSections.length > 0 && (
                  <div className="rounded-xl border bg-card overflow-hidden m-4 mt-2">
                    <div className="flex items-center gap-2 px-4 py-3 border-b bg-muted/30">
                      <Settings className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm font-semibold">{t('componentConfig.configOptions')}</span>
                    </div>
                    <div className="p-4">
                      <ConfigRenderer sections={allSections} />
                    </div>
                  </div>
                )}
              </TabsContent>
            </Tabs>
          </div>

          {/* Large screens: Two-column layout */}
          <div className="hidden lg:flex flex-1 overflow-hidden">
            {/* Left: Preview + Style/Display Config (50% or 100%) */}
            <div className={cn(
              hasDataSource ? "w-1/2 border-r" : "w-full",
              "flex flex-col bg-background overflow-hidden"
            )}>
              {/* Preview */}
              <div className="shrink-0 border-b overflow-hidden">
                <ComponentPreview
                  key={previewKey}
                  componentType={componentType}
                  config={livePreviewConfig}
                  dataSource={livePreviewDataSource}
                  title={title}
                  showHeader={true}
                />
              </div>

              {/* Config Tabs - Style, Display */}
              {(hasStyleConfig || hasDisplayConfig) && (
                <Tabs value={configTabValue} onValueChange={(v) => setConfigTabValue(v as 'style' | 'display')} className="flex-1 flex flex-col min-h-0">
                  <TabsList className="w-full justify-start rounded-none border-b bg-transparent px-3 h-10 shrink-0">
                    {hasStyleConfig && (
                      <TabsTrigger value="style" className="data-[state=active]:bg-transparent data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none">
                        {t('componentConfig.style')}
                      </TabsTrigger>
                    )}
                    {hasDisplayConfig && (
                      <TabsTrigger value="display" className="data-[state=active]:bg-transparent data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none">
                        {t('componentConfig.display')}
                      </TabsTrigger>
                    )}
                  </TabsList>

                  {hasStyleConfig && (
                    <TabsContent value="style" className="min-h-0 overflow-y-auto p-3">
                      <ConfigRenderer sections={filteredStyleSections} />
                    </TabsContent>
                  )}

                  {hasDisplayConfig && (
                    <TabsContent value="display" className="min-h-0 overflow-y-auto p-3">
                      <ConfigRenderer sections={finalDisplaySections} />
                    </TabsContent>
                  )}
                </Tabs>
              )}

              {/* Legacy sections fallback */}
              {!hasStyleConfig && !hasDisplayConfig && allSections.length > 0 && !hasDataSource && (
                <div className="flex-1 overflow-y-auto p-3">
                  <ConfigRenderer sections={allSections} />
                </div>
              )}
            </div>

            {/* Right: Data Source + Transform Config (50%) - Only show when hasDataSource */}
            {hasDataSource && (
              <div className="w-1/2 flex flex-col overflow-hidden bg-background">
              {/* Data Source + Transform Section */}
              <div className="flex-1 min-h-0 flex flex-col">
                {hasDataSource ? (
                  <>
                    {/* Tab List - External wrapper */}
                    <div className="flex items-center gap-2 px-4 py-2 bg-muted/30 border-b shrink-0">
                      <div className="flex gap-1">
                        <button
                          onClick={() => setRightDataSourceTab('datasource')}
                          className={`px-3 h-8 text-sm rounded-none transition-colors border-b-2 ${
                            rightDataSourceTab === 'datasource'
                              ? 'border-primary text-foreground'
                              : 'border-transparent text-muted-foreground hover:text-foreground'
                          }`}
                        >
                          {t('componentConfig.dataSource')}
                        </button>
                        {shouldShowDataTransform && (
                          <button
                            onClick={() => setRightDataSourceTab('transform')}
                            className={`px-3 h-8 text-sm rounded-none transition-colors border-b-2 ${
                              rightDataSourceTab === 'transform'
                                ? 'border-primary text-foreground'
                                : 'border-transparent text-muted-foreground hover:text-foreground'
                            }`}
                          >
                            {t('componentConfig.transform')}
                          </button>
                        )}
                      </div>
                      {hasConfiguredDataSource && (
                        <CheckCircle2 className="h-4 w-4 text-green-500 ml-auto" />
                      )}
                    </div>

                    {/* Tab Content - Direct rendering without Tabs wrapper */}
                    <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
                      {rightDataSourceTab === 'datasource' ? (
                        <UnifiedDataSourceConfig
                          value={previewDataSource}
                          onChange={handleDataSourceChange}
                          allowedTypes={dataSourceProps?.allowedTypes}
                          multiple={multiple}
                          maxSources={maxSources}
                        />
                      ) : (
                        <div className="flex-1 overflow-y-auto p-4">
                          <DataTransformConfig
                            dataSource={previewDataSource}
                            onChange={handleDataTransformChange}
                            chartType={getChartTypeForTransform(componentType)}
                          />
                        </div>
                      )}
                    </div>

                    {/* Other data source related sections */}
                    {dataSourceSections.length > 1 && (
                      <div className="px-5 py-3 border-t shrink-0">
                        <ConfigRenderer sections={dataSourceSections.slice(1)} />
                      </div>
                    )}
                  </>
                ) : (
                  <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground">
                    <div className="w-12 h-12 rounded-xl bg-muted/30 flex items-center justify-center mb-3">
                      <Settings className="h-6 w-6 opacity-40" />
                    </div>
                    <p className="text-sm">{t('componentConfig.noDataSourceNeeded')}</p>
                  </div>
                )}
              </div>
            </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t flex justify-end gap-3 shrink-0 bg-muted/20">
          <Button
            variant="outline"
            onClick={onClose}
            className="h-10 px-5 rounded-lg"
          >
            {t('common.cancel')}
          </Button>
          <Button
            onClick={onSave}
            className="h-10 px-5 rounded-lg"
          >
            {t('common.saveChanges')}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
