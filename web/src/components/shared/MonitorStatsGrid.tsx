import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export interface MonitorStat {
  label: string;
  value: string | number;
  icon?: React.ReactNode;
  color?: "default" | "info" | "success" | "warning" | "error" | "purple";
}

interface MonitorStatsGridProps {
  stats: MonitorStat[];
  className?: string;
}

const colorClasses = {
  default: "bg-muted text-foreground border-border",
  info: "bg-info-light text-info border-info",
  success: "bg-success-light text-success border-success",
  warning: "bg-warning-light text-warning border-warning",
  error: "bg-error-light text-error border-error",
  purple: "bg-purple-50 text-purple-700 border-purple-200 dark:bg-purple-950 dark:text-purple-300 dark:border-purple-900",
};

export function MonitorStatsGrid({ stats, className }: MonitorStatsGridProps) {
  return (
    <div className={cn("grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3", className)}>
      {stats.map((stat, index) => (
        <Card
          key={index}
          className={cn(
            "border p-3 transition-colors hover:shadow-md",
            colorClasses[stat.color || "default"]
          )}
        >
          <div className="flex items-center justify-between">
            <div className="flex-1 min-w-0">
              <p className="text-xs font-medium text-muted-foreground truncate">
                {stat.label}
              </p>
              <p className="text-lg font-semibold truncate">
                {stat.value}
              </p>
            </div>
            {stat.icon && (
              <div className="ml-2 flex-shrink-0 text-muted-foreground">
                {stat.icon}
              </div>
            )}
          </div>
        </Card>
      ))}
    </div>
  );
}
