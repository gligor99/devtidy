import type { Ecosystem } from '@/bindings'
import { ECOSYSTEM_LABELS, formatBytes } from '@/lib/format'
import { cn } from '@/lib/utils'

export type View = { kind: 'items'; ecosystem: Ecosystem | null } | { kind: 'history' }

type Props = {
  view: View
  onChange: (view: View) => void
  totals: Map<Ecosystem, number>
}

function NavButton({
  active,
  label,
  detail,
  onClick,
}: {
  active: boolean
  label: string
  detail?: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={active ? 'page' : undefined}
      className={cn(
        'flex w-full items-center justify-between rounded-md px-2.5 py-1.5 text-left text-[13px] transition-colors',
        active ? 'bg-sidebar-accent text-sidebar-accent-foreground font-medium' : 'hover:bg-sidebar-accent/60',
      )}
    >
      <span>{label}</span>
      {detail && <span className="text-muted-foreground text-xs tabular-nums">{detail}</span>}
    </button>
  )
}

export function Sidebar({ view, onChange, totals }: Props) {
  const ecosystems = [...totals.entries()].sort((a, b) => b[1] - a[1])
  const isItems = (eco: Ecosystem | null) => view.kind === 'items' && view.ecosystem === eco

  return (
    <nav className="bg-sidebar text-sidebar-foreground flex w-56 shrink-0 flex-col border-r">
      {/* Room for the macOS traffic lights; also drags the window. */}
      <div data-tauri-drag-region className="h-12 shrink-0" />
      <div className="flex flex-col gap-0.5 px-2">
        <NavButton active={isItems(null)} label="Overview" onClick={() => onChange({ kind: 'items', ecosystem: null })} />
      </div>
      <p className="text-muted-foreground px-4 pt-4 pb-1 text-[11px] font-medium tracking-wide uppercase">
        Ecosystems
      </p>
      <div className="flex flex-col gap-0.5 px-2">
        {ecosystems.map(([eco, bytes]) => (
          <NavButton
            key={eco}
            active={isItems(eco)}
            label={ECOSYSTEM_LABELS[eco]}
            detail={formatBytes(bytes)}
            onClick={() => onChange({ kind: 'items', ecosystem: eco })}
          />
        ))}
      </div>
      <div className="mt-auto flex flex-col gap-0.5 px-2 pb-3">
        <NavButton active={view.kind === 'history'} label="History" onClick={() => onChange({ kind: 'history' })} />
      </div>
    </nav>
  )
}
