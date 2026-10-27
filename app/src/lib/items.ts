import type { Ecosystem, Finding, PreviousSize, Rule_Serialize as Rule, Safety } from '@/bindings'

/** A rule joined with what the latest scan found for it. */
export type Item = {
  rule: Rule
  finding: Finding | null
  previous: PreviousSize | null
}

export type ItemFilter = { kind: 'all' } | { kind: 'ecosystem'; ecosystem: Ecosystem }

export function isPresent(item: Item): boolean {
  return item.finding !== null && item.finding.status !== 'missing'
}

export function allocated(item: Item): number {
  return item.finding?.size.allocated ?? 0
}

export function reclaimable(item: Item): number {
  return item.finding?.size.reclaimable ?? 0
}

/** Growth since the previous scan, or null without history. */
export function growth(item: Item): number | null {
  if (!item.previous || !item.finding) return null
  return item.finding.size.allocated - item.previous.allocated
}

export function buildItems(
  rules: Rule[],
  findings: Finding[],
  previous: Partial<Record<string, PreviousSize>>,
): Item[] {
  const byId = new Map(findings.map((f) => [f.rule_id, f]))
  return rules.map((rule) => ({
    rule,
    finding: byId.get(rule.id) ?? null,
    previous: previous[rule.id] ?? null,
  }))
}

/** Present items matching the filter, largest first. */
export function visibleItems(items: Item[], filter: ItemFilter): Item[] {
  return items
    .filter(isPresent)
    .filter((i) => filter.kind === 'all' || i.rule.ecosystem === filter.ecosystem)
    .sort((a, b) => allocated(b) - allocated(a) || a.rule.id.localeCompare(b.rule.id))
}

export function totalsBySafety(items: Item[]): Record<Safety | 'all', number> {
  const totals = { all: 0, safe: 0, caution: 0, dangerous: 0 }
  for (const item of items) {
    const bytes = reclaimable(item)
    totals.all += bytes
    totals[item.rule.safety] += bytes
  }
  return totals
}

export function ecosystemTotals(items: Item[]): Map<Ecosystem, number> {
  const totals = new Map<Ecosystem, number>()
  for (const item of items.filter(isPresent)) {
    totals.set(item.rule.ecosystem, (totals.get(item.rule.ecosystem) ?? 0) + allocated(item))
  }
  return totals
}

/**
 * Toggles an item in the selection, mirroring the backend rule that a
 * dangerous item can only be cleaned on its own.
 */
export function toggleSelection(selected: ReadonlySet<string>, item: Item, items: Item[]): Set<string> {
  const id = item.rule.id
  if (selected.has(id)) {
    const next = new Set(selected)
    next.delete(id)
    return next
  }
  if (item.rule.safety === 'dangerous') return new Set([id])
  const dangerous = new Set(items.filter((i) => i.rule.safety === 'dangerous').map((i) => i.rule.id))
  return new Set([...selected].filter((s) => !dangerous.has(s))).add(id)
}

/** Every present safe item, the CLI's `--safe`. */
export function selectAllSafe(items: Item[]): Set<string> {
  return new Set(items.filter((i) => isPresent(i) && i.rule.safety === 'safe').map((i) => i.rule.id))
}

export function describeCleanup(rule: Rule): string {
  const removal =
    rule.safety === 'safe'
      ? 'Deleted permanently'
      : rule.safety === 'caution'
        ? 'Moved to the Trash'
        : 'Moved to the Trash, only on its own'
  const cleanup = rule.cleanup
  if (cleanup.kind === 'command') {
    const run = `Runs \`${cleanup.argv.join(' ')}\``
    return cleanup.fallback_remove ? `${run} (if not installed: ${removal.toLowerCase()})` : run
  }
  return cleanup.contents_only ? `${removal} (contents only, the folder is kept)` : removal
}
