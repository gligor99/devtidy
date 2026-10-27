import { useMutation, useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import type { Method, Plan, Report_Serialize as Report } from '@/bindings'
import { SafetyBadge } from '@/components/safety-badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import * as api from '@/lib/api'
import { formatBytes } from '@/lib/format'

type Props = {
  ids: string[]
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Called after a cleanup ran, whatever its outcome. */
  onCleaned: (report: Report) => void
}

function methodLabel(method: Method): string {
  switch (method.kind) {
    case 'delete':
      return 'Delete'
    case 'trash':
      return 'Move to Trash'
    case 'command':
      return `Run ${method.argv.join(' ')}`
  }
}

function PlanView({ plan }: { plan: Plan }) {
  return (
    <div className="flex flex-col gap-3">
      <ul className="divide-y rounded-md border">
        {plan.actions.map((action) => (
          <li key={action.rule_id} className="flex items-center gap-3 px-3 py-2 text-[13px]">
            <div className="min-w-0 flex-1">
              <p className="truncate font-medium">{action.name}</p>
              <p className="text-muted-foreground text-xs">
                {action.blocked_by.length > 0
                  ? `Skipped: quit ${action.blocked_by.join(', ')} first`
                  : methodLabel(action.method)}
              </p>
            </div>
            <SafetyBadge safety={action.safety} />
            <span className="w-20 text-right tabular-nums">{formatBytes(action.estimated)}</span>
          </li>
        ))}
      </ul>
      {plan.skipped.length > 0 && (
        <ul className="text-muted-foreground text-xs">
          {plan.skipped.map((s) => (
            <li key={s.rule_id}>
              Skipping {s.rule_id}: {s.reason}.
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

function ReportView({ report }: { report: Report }) {
  const failed = report.results.filter((r) => r.outcome.result === 'failed')
  return (
    <div className="flex flex-col gap-2 text-[13px]">
      {report.freed !== null && (
        <p>
          Freed <strong>{formatBytes(report.freed)}</strong> (measured on disk).
        </p>
      )}
      {report.trashed > 0 && (
        <p>Moved {formatBytes(report.trashed)} to the Trash. Empty it to free that space.</p>
      )}
      {failed.map((r) => (
        <p key={r.rule_id} className="text-destructive">
          {r.rule_id}: {r.outcome.result === 'failed' ? r.outcome.error : ''}
        </p>
      ))}
    </div>
  )
}

export function CleanDialog({ ids, open, onOpenChange, onCleaned }: Props) {
  const [typed, setTyped] = useState('')
  const plan = useQuery({
    queryKey: ['plan', ids],
    queryFn: () => api.planClean(ids),
    enabled: open && ids.length > 0,
    gcTime: 0,
  })
  const clean = useMutation({ mutationFn: () => api.clean(ids), onSuccess: (report) => onCleaned(report) })

  const runnable = plan.data?.actions.filter((a) => a.blocked_by.length === 0) ?? []
  const dangerous = runnable.find((a) => a.safety === 'dangerous')
  const confirmed = !dangerous || typed === dangerous.rule_id
  const estimated = runnable.reduce((sum, a) => sum + a.estimated, 0)

  const close = (next: boolean) => {
    if (clean.isPending) return
    if (!next) {
      setTyped('')
      clean.reset()
    }
    onOpenChange(next)
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{clean.data ? 'Cleanup finished' : 'Review cleanup'}</DialogTitle>
          <DialogDescription>
            {clean.data
              ? 'Here is what changed on disk.'
              : 'Nothing is changed until you confirm. Items are re-checked right before removal.'}
          </DialogDescription>
        </DialogHeader>

        {clean.data ? (
          <ReportView report={clean.data} />
        ) : plan.isPending ? (
          <p className="text-muted-foreground text-sm">Preparing plan…</p>
        ) : plan.error ? (
          <p className="text-destructive text-sm">{plan.error.message}</p>
        ) : plan.data ? (
          <PlanView plan={plan.data} />
        ) : null}

        {dangerous && !clean.data && (
          <label className="flex flex-col gap-1.5 text-[13px]">
            <span>
              <strong>{dangerous.name}</strong> cannot be fully regenerated. Type{' '}
              <code className="font-mono">{dangerous.rule_id}</code> to confirm.
            </span>
            <Input value={typed} onChange={(e) => setTyped(e.target.value)} autoFocus aria-label="Confirm id" />
          </label>
        )}
        {clean.error && <p className="text-destructive text-sm">{clean.error.message}</p>}

        <DialogFooter>
          {clean.data ? (
            <Button onClick={() => close(false)}>Done</Button>
          ) : (
            <>
              <Button variant="outline" onClick={() => close(false)} disabled={clean.isPending}>
                Cancel
              </Button>
              <Button
                variant={dangerous ? 'destructive' : 'default'}
                disabled={runnable.length === 0 || !confirmed || clean.isPending}
                onClick={() => clean.mutate()}
              >
                {clean.isPending ? 'Cleaning…' : `Clean ${formatBytes(estimated)}`}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
