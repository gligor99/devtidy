import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useMemo, useState } from 'react'
import { toast } from 'sonner'
import type { Report_Serialize as Report } from '@/bindings'
import { CleanDialog } from '@/components/clean-dialog'
import { DiskHeader } from '@/components/disk-header'
import { HistoryView } from '@/components/history-view'
import { ItemDetails } from '@/components/item-details'
import { ItemList } from '@/components/item-list'
import { LogoMark } from '@/components/logo'
import { Sidebar, type View } from '@/components/sidebar'
import { Button } from '@/components/ui/button'
import * as api from '@/lib/api'
import { formatBytes } from '@/lib/format'
import {
  buildItems,
  ecosystemTotals,
  reclaimable,
  selectAllSafe,
  toggleSelection,
  totalsBySafety,
  visibleItems,
} from '@/lib/items'
import { useRules, useScan } from '@/lib/use-scan'

export default function App() {
  const queryClient = useQueryClient()
  const rules = useRules()
  const scan = useScan()
  const disk = useQuery({ queryKey: ['disk'], queryFn: api.getDisk })

  const [view, setView] = useState<View>({ kind: 'items', ecosystem: null })
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [focusedId, setFocusedId] = useState<string | null>(null)
  const [cleaning, setCleaning] = useState(false)

  const items = useMemo(
    () => buildItems(rules.data ?? [], scan.findings, scan.result?.previous ?? {}),
    [rules.data, scan.findings, scan.result],
  )
  const visible = visibleItems(
    items,
    view.kind === 'items' && view.ecosystem ? { kind: 'ecosystem', ecosystem: view.ecosystem } : { kind: 'all' },
  )
  const totals = totalsBySafety(visibleItems(items, { kind: 'all' }))
  const focused = items.find((i) => i.rule.id === focusedId) ?? null
  const selectedBytes = items.filter((i) => selected.has(i.rule.id)).reduce((sum, i) => sum + reclaimable(i), 0)

  const onCleaned = (report: Report) => {
    setSelected(new Set())
    void queryClient.invalidateQueries({ queryKey: ['history'] })
    void queryClient.invalidateQueries({ queryKey: ['disk'] })
    scan.rescan()
    if (report.freed !== null) toast.success(`Freed ${formatBytes(report.freed)}`)
  }

  return (
    <div className="flex h-screen">
      <Sidebar view={view} onChange={setView} totals={ecosystemTotals(items)} />
      <main className="flex min-w-0 flex-1 flex-col">
        <DiskHeader
          disk={scan.result?.disk ?? disk.data ?? null}
          reclaimable={totals}
          progress={scan.progress}
          onRescan={scan.rescan}
        />
        {view.kind === 'history' ? (
          <div className="flex-1 overflow-y-auto">
            <HistoryView />
          </div>
        ) : (
          <div className="flex min-h-0 flex-1">
            <div className="flex min-w-0 flex-1 flex-col">
              {scan.error && <p className="text-destructive px-6 py-3 text-sm">{scan.error.message}</p>}
              <div className="flex-1 overflow-y-auto">
                {scan.scanning && visible.length === 0 ? (
                  <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
                    <LogoMark size={56} className="animate-pulse" />
                    <p className="text-muted-foreground text-sm">Looking through your developer tooling…</p>
                  </div>
                ) : (
                  <ItemList
                    items={visible}
                    selected={selected}
                    focused={focusedId}
                    onToggle={(item) => setSelected((s) => toggleSelection(s, item, items))}
                    onFocus={(item) => setFocusedId(item.rule.id)}
                  />
                )}
              </div>
              <footer className="flex shrink-0 items-center justify-between gap-3 border-t px-6 py-3">
                <Button variant="ghost" size="sm" onClick={() => setSelected(selectAllSafe(items))}>
                  Select all safe
                </Button>
                <Button disabled={selected.size === 0 || scan.scanning} onClick={() => setCleaning(true)}>
                  {selected.size === 0 ? 'Clean selected' : `Clean ${selected.size} · ${formatBytes(selectedBytes)}`}
                </Button>
              </footer>
            </div>
            <ItemDetails item={focused} />
          </div>
        )}
      </main>
      <CleanDialog ids={[...selected]} open={cleaning} onOpenChange={setCleaning} onCleaned={onCleaned} />
    </div>
  )
}
