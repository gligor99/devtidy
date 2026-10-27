import { SafetyBadge } from '@/components/safety-badge'
import { Checkbox } from '@/components/ui/checkbox'
import { ECOSYSTEM_LABELS, formatBytes, formatDelta } from '@/lib/format'
import { type Item, allocated, growth, reclaimable } from '@/lib/items'
import { cn } from '@/lib/utils'

type Props = {
  items: Item[]
  selected: ReadonlySet<string>
  focused: string | null
  onToggle: (item: Item) => void
  onFocus: (item: Item) => void
}

function SizeCell({ item }: { item: Item }) {
  const status = item.finding?.status
  if (status === 'denied') return <span className="text-muted-foreground">No access</span>
  const size = allocated(item)
  const frees = reclaimable(item)
  const delta = growth(item)
  const deltaText = delta === null ? null : formatDelta(delta)
  return (
    <div className="flex flex-col items-end">
      <span className="tabular-nums">
        {formatBytes(size)}
        {status === 'partial' && <span title="Some files could not be read">+</span>}
      </span>
      {frees < size ? (
        <span className="text-muted-foreground text-[11px]">frees ~{formatBytes(frees)}</span>
      ) : deltaText ? (
        <span className={cn('text-[11px] tabular-nums', delta! > 0 ? 'text-caution' : 'text-safe')}>
          {deltaText} since last scan
        </span>
      ) : null}
    </div>
  )
}

export function ItemList({ items, selected, focused, onToggle, onFocus }: Props) {
  if (items.length === 0) {
    return <p className="text-muted-foreground p-8 text-center text-sm">Nothing found here.</p>
  }
  return (
    <ul role="listbox" aria-label="Items" aria-multiselectable className="divide-y">
      {items.map((item) => {
        const id = item.rule.id
        const canSelect = item.finding?.status !== 'denied'
        return (
          <li
            key={id}
            role="option"
            aria-selected={selected.has(id)}
            onClick={() => onFocus(item)}
            className={cn(
              'flex items-center gap-3 px-6 py-2.5',
              focused === id ? 'bg-accent' : 'hover:bg-accent/50',
            )}
          >
            <Checkbox
              aria-label={`Select ${item.rule.name}`}
              checked={selected.has(id)}
              disabled={!canSelect}
              onCheckedChange={() => onToggle(item)}
              onClick={(e) => e.stopPropagation()}
            />
            <div className="min-w-0 flex-1">
              <p className="truncate text-[13px] font-medium">{item.rule.name}</p>
              <p className="text-muted-foreground text-xs">{ECOSYSTEM_LABELS[item.rule.ecosystem]}</p>
            </div>
            <SafetyBadge safety={item.rule.safety} />
            <div className="w-36 text-right text-[13px]">
              <SizeCell item={item} />
            </div>
          </li>
        )
      })}
    </ul>
  )
}
