import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { FolderSearchIcon } from 'lucide-react'
import { SafetyBadge } from '@/components/safety-badge'
import { Button } from '@/components/ui/button'
import { formatAgo, formatBytes } from '@/lib/format'
import { type Item, allocated, describeCleanup, reclaimable } from '@/lib/items'

const REGEN_LABELS = { low: 'Low', medium: 'Medium', high: 'High' } as const

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <dt className="text-muted-foreground text-[11px] font-medium tracking-wide uppercase">{label}</dt>
      <dd className="selectable text-[13px] leading-relaxed">{children}</dd>
    </div>
  )
}

export function ItemDetails({ item }: { item: Item | null }) {
  if (!item) {
    return (
      <aside className="text-muted-foreground flex w-80 shrink-0 items-center justify-center border-l p-6 text-center text-sm">
        Select an item to see what it is and what removing it costs.
      </aside>
    )
  }
  const { rule, finding, previous } = item
  const size = allocated(item)
  const frees = reclaimable(item)

  return (
    <aside className="flex w-80 shrink-0 flex-col gap-5 overflow-y-auto border-l p-6">
      <div className="flex flex-col gap-2">
        <SafetyBadge safety={rule.safety} className="self-start" />
        <h2 className="selectable text-base font-semibold">{rule.name}</h2>
        <p className="selectable text-muted-foreground text-[13px] leading-relaxed">{rule.description}</p>
      </div>
      <dl className="flex flex-col gap-4">
        <Field label="Size">
          {formatBytes(size)}
          {frees < size && <> · frees ~{formatBytes(frees)} (files are shared via hard links)</>}
          {previous && (
            <span className="text-muted-foreground block text-xs">
              {formatBytes(previous.allocated)} {formatAgo(previous.at)}
            </span>
          )}
        </Field>
        <Field label="After removal">{rule.regen_note}</Field>
        <Field label="Cost to regenerate">{REGEN_LABELS[rule.regen_cost]}</Field>
        <Field label="How it is removed">{describeCleanup(rule)}</Field>
        {rule.processes.length > 0 && <Field label="Quit first">{rule.processes.join(', ')}</Field>}
        <Field label="Location">
          <ul className="flex flex-col gap-1">
            {finding?.paths.map((path) => (
              <li key={path} className="flex items-start gap-1">
                <span className="min-w-0 flex-1 font-mono text-xs break-all">{path}</span>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  aria-label="Show in Finder"
                  title="Show in Finder"
                  onClick={() => void revealItemInDir(path)}
                >
                  <FolderSearchIcon />
                </Button>
              </li>
            ))}
          </ul>
        </Field>
        {rule.docs && (
          <Field label="Docs">
            <span className="font-mono text-xs break-all">{rule.docs}</span>
          </Field>
        )}
      </dl>
    </aside>
  )
}
