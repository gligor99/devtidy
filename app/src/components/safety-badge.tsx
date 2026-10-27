import type { Safety } from '@/bindings'
import { cn } from '@/lib/utils'

const STYLES: Record<Safety, string> = {
  safe: 'text-safe bg-safe/10',
  caution: 'text-caution bg-caution/12',
  dangerous: 'text-dangerous bg-dangerous/10',
}

export function SafetyBadge({ safety, className }: { safety: Safety; className?: string }) {
  return (
    <span
      className={cn(
        'inline-flex h-5 items-center rounded-full px-2 text-[11px] font-medium capitalize',
        STYLES[safety],
        className,
      )}
    >
      {safety}
    </span>
  )
}
