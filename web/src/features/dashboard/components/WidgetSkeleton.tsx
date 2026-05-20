/**
 * WidgetSkeleton — loading placeholder for widgets
 */

import { Skeleton } from '@/components/ui/skeleton'

export function WidgetSkeleton() {
  return (
    <div className="w-full h-full p-3">
      <Skeleton className="w-1/3 h-4 mb-3" />
      <Skeleton className="w-full h-[calc(100%-28px)] rounded-md" />
    </div>
  )
}
