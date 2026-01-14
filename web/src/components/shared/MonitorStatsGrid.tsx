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
  default: "bg-slate-50 text-slate-700 border-slate-200 dark:bg-slate-900 dark:text-slate-300 dark:border-slate-800",
  info: "bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-950 dark:text-blue-300 dark:border-blue-900",
  success: "bg-green-50 text-green-700 border-green-200 dark:bg-green-950 dark:text-green-300 dark:border-green-900",
  warning: "bg-yellow-50 text-yellow-700 border-yellow-200 dark:bg-yellow-950 dark:text-yellow-300 dark:border-yellow-900",
  error: "bg-red-50 text-red-700 border-red-200 dark:bg-red-950 dark:text-red-300 dark:border-red-900",
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
              <div className="ml-2 flex-shrink-0 text-muted-foreground/70">
                {stat.icon}
              </div>
            )}
          </div>
        </Card>
      ))}
    </div>
  );
}
