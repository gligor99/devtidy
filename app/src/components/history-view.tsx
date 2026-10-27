import { useQuery } from '@tanstack/react-query'
import * as api from '@/lib/api'
import { formatAgo, formatBytes } from '@/lib/format'

export function HistoryView() {
  const history = useQuery({ queryKey: ['history'], queryFn: () => api.getHistory(50) })

  if (history.isPending) return <p className="text-muted-foreground p-8 text-sm">Loading…</p>
  if (history.error) return <p className="text-destructive p-8 text-sm">{history.error.message}</p>
  if (history.data.length === 0) {
    return <p className="text-muted-foreground p-8 text-center text-sm">No cleanups yet.</p>
  }

  return (
    <ul className="divide-y">
      {history.data.map((record) => {
        const done = record.actions.filter((a) => a.outcome === 'done')
        return (
          <li key={record.id} className="flex flex-col gap-1 px-6 py-3 text-[13px]">
            <div className="flex justify-between">
              <span className="font-medium">
                {record.freed !== null ? `Freed ${formatBytes(record.freed)}` : 'Cleanup'}
                {record.trashed > 0 && ` · ${formatBytes(record.trashed)} to Trash`}
              </span>
              <span className="text-muted-foreground text-xs">{formatAgo(record.at)}</span>
            </div>
            <p className="text-muted-foreground text-xs">
              {done.map((a) => a.rule_id).join(', ') || 'Nothing was removed'}
              {done.length < record.actions.length && ` · ${record.actions.length - done.length} skipped or failed`}
            </p>
          </li>
        )
      })}
    </ul>
  )
}
