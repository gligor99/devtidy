import { RefreshCwIcon } from 'lucide-react'
import type { DiskSpace } from '@/bindings'
import { Button } from '@/components/ui/button'
import { formatBytes } from '@/lib/format'

type Props = {
  disk: DiskSpace | null
  reclaimable: { all: number; safe: number }
  progress: { done: number; total: number } | null
  onRescan: () => void
}

export function DiskHeader({ disk, reclaimable, progress, onRescan }: Props) {
  const used = disk ? disk.total - disk.available : 0
  const usedPct = disk && disk.total > 0 ? (used / disk.total) * 100 : 0
  const reclaimPct = disk && disk.total > 0 ? (reclaimable.all / disk.total) * 100 : 0
  const scanning = progress !== null

  return (
    <header data-tauri-drag-region className="flex shrink-0 flex-col gap-3 border-b px-6 pt-4 pb-4">
      <div data-tauri-drag-region className="flex items-end justify-between gap-4">
        <div data-tauri-drag-region>
          <p className="text-muted-foreground text-xs">Can be reclaimed</p>
          <p className="text-2xl font-semibold tabular-nums">
            {formatBytes(reclaimable.all)}
            <span className="text-muted-foreground ml-2 text-sm font-normal">
              {formatBytes(reclaimable.safe)} marked safe
            </span>
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={onRescan} disabled={scanning}>
          <RefreshCwIcon className={scanning ? 'animate-spin' : undefined} />
          {scanning ? `Scanning ${progress.done}/${progress.total || '…'}` : 'Rescan'}
        </Button>
      </div>
      {disk && (
        <div>
          <div
            className="bg-muted relative h-2 overflow-hidden rounded-full"
            role="meter"
            aria-label="Disk usage"
            aria-valuenow={Math.round(usedPct)}
            aria-valuemin={0}
            aria-valuemax={100}
          >
            <div className="bg-foreground/70 absolute inset-y-0 left-0" style={{ width: `${usedPct}%` }} />
            <div
              className="bg-safe absolute inset-y-0"
              style={{ left: `${Math.max(0, usedPct - reclaimPct)}%`, width: `${reclaimPct}%` }}
            />
          </div>
          <p className="text-muted-foreground mt-1.5 text-xs tabular-nums">
            {formatBytes(used)} of {formatBytes(disk.total)} used · {formatBytes(disk.available)} free
          </p>
        </div>
      )}
    </header>
  )
}
