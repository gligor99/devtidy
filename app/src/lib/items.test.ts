import { finding, item, rule } from '@/test/fixtures'
import { formatAgo, formatBytes, formatDelta } from './format'
import { buildItems, describeCleanup, growth, selectAllSafe, toggleSelection, totalsBySafety, visibleItems } from './items'

describe('formatting', () => {
  it('uses SI units like Finder', () => {
    expect(formatBytes(999)).toBe('999 B')
    expect(formatBytes(18_200_000_000)).toBe('18.2 GB')
  })

  it('hides tiny deltas and signs the rest', () => {
    expect(formatDelta(500_000)).toBeNull()
    expect(formatDelta(4_000_000_000)).toBe('+4.0 GB')
    expect(formatDelta(-2_500_000)).toBe('−2.5 MB')
  })

  it('formats relative times', () => {
    expect(formatAgo(1000, 1030)).toBe('just now')
    expect(formatAgo(0, 3 * 86_400)).toBe('3 days ago')
  })
})

describe('items', () => {
  it('joins rules with findings and previous sizes', () => {
    const items = buildItems([rule('a'), rule('b')], [finding('a', 10)], { a: { at: 1, allocated: 4 } })
    expect(items[0]!.finding?.size.allocated).toBe(10)
    expect(growth(items[0]!)).toBe(6)
    expect(items[1]!.finding).toBeNull()
  })

  it('shows present items largest first, filtered by ecosystem', () => {
    const items = [item('small', 'safe', 1), item('big', 'safe', 9), { ...item('gone'), finding: null }]
    expect(visibleItems(items, { kind: 'all' }).map((i) => i.rule.id)).toEqual(['big', 'small'])
    expect(visibleItems(items, { kind: 'ecosystem', ecosystem: 'ios' })).toEqual([])
  })

  it('totals reclaimable bytes by safety', () => {
    const totals = totalsBySafety([item('a', 'safe', 5), item('b', 'caution', 7)])
    expect(totals).toEqual({ all: 12, safe: 5, caution: 7, dangerous: 0 })
  })
})

describe('selection', () => {
  const safe = item('safe.one')
  const caution = item('caution.one', 'caution')
  const dangerous = item('dangerous.one', 'dangerous')
  const all = [safe, caution, dangerous]

  it('adds and removes ids', () => {
    const one = toggleSelection(new Set(), safe, all)
    expect([...one]).toEqual(['safe.one'])
    expect(toggleSelection(one, safe, all).size).toBe(0)
  })

  it('keeps a dangerous item alone, like the backend requires', () => {
    const mixed = toggleSelection(new Set(['safe.one', 'caution.one']), dangerous, all)
    expect([...mixed]).toEqual(['dangerous.one'])
    expect([...toggleSelection(mixed, safe, all)]).toEqual(['safe.one'])
  })

  it('select all safe never picks caution or dangerous', () => {
    expect([...selectAllSafe(all)]).toEqual(['safe.one'])
  })
})

describe('describeCleanup', () => {
  it('mirrors the removal strategy', () => {
    expect(describeCleanup(rule('a', 'safe', { cleanup: { kind: 'remove', contents_only: true } }))).toBe(
      'Deleted permanently (contents only, the folder is kept)',
    )
    expect(describeCleanup(rule('a', 'caution'))).toBe('Moved to the Trash')
    expect(
      describeCleanup(rule('a', 'safe', { cleanup: { kind: 'command', argv: ['npm', 'cache', 'clean'], fallback_remove: true } })),
    ).toBe('Runs `npm cache clean` (if not installed: deleted permanently)')
  })
})
