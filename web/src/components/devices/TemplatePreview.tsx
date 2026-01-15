// TemplatePreview Component
//
// Displays a preview of device type template capabilities (metrics and commands)

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Label } from "@/components/ui/label"
import type { DeviceType } from "@/types"

interface TemplatePreviewProps {
  template: DeviceType
  className?: string
}

export function TemplatePreview({ template, className }: TemplatePreviewProps) {
  // Get metrics from simplified format or legacy format
  const metrics = template.metrics || []
  
  // Get commands from simplified format or legacy format
  const commands = template.commands || []

  return (
    <Card className={className}>
      <CardHeader>
        <CardTitle>{template.name}</CardTitle>
        <CardDescription>
          {template.description || template.device_type}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Categories */}
        {template.categories && template.categories.length > 0 && (
          <div>
            <Label className="text-xs text-muted-foreground">Categories</Label>
            <div className="mt-1 flex flex-wrap gap-1">
              {template.categories.map((cat, i) => (
                <Badge key={i} variant="secondary" className="text-xs">
                  {cat}
                </Badge>
              ))}
            </div>
          </div>
        )}

        {/* Metrics Preview */}
        <div>
          <Label className="text-xs text-muted-foreground">
            Metrics ({metrics.length})
          </Label>
          <div className="mt-2 space-y-1">
            {metrics.length > 0 ? (
              metrics.map((metric, i) => (
                <Badge
                  key={i}
                  variant="outline"
                  className="mr-1 mb-1 text-xs"
                >
                  {metric.display_name || metric.name}
                  <span className="ml-1 text-muted-foreground">
                    ({metric.data_type}
                    {metric.unit && `, ${metric.unit}`})
                  </span>
                </Badge>
              ))
            ) : (
              <p className="text-xs text-muted-foreground">No metrics defined</p>
            )}
          </div>
        </div>

        {/* Commands Preview */}
        {commands.length > 0 && (
          <div>
            <Label className="text-xs text-muted-foreground">
              Commands ({commands.length})
            </Label>
            <div className="mt-2 space-y-1">
              {commands.map((cmd, i) => (
                <Badge
                  key={i}
                  variant="outline"
                  className="mr-1 mb-1 text-xs"
                >
                  {cmd.display_name || cmd.name}
                  {cmd.parameters && cmd.parameters.length > 0 && (
                    <span className="ml-1 text-muted-foreground">
                      ({cmd.parameters.length} params)
                    </span>
                  )}
                </Badge>
              ))}
            </div>
          </div>
        )}

        {/* Summary */}
        <div className="pt-2 border-t">
          <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
            <div>
              <span className="font-medium">{metrics.length}</span> metrics
            </div>
            <div>
              <span className="font-medium">{commands.length}</span> commands
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}