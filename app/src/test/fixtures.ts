import type { Finding, Rule_Serialize as Rule, Safety } from '@/bindings'
import type { Item } from '@/lib/items'

export function rule(id: string, safety: Safety = 'safe', overrides: Partial<Rule> = {}): Rule {
  return {
    id,
    name: id,
    ecosystem: 'android',
    platforms: [],
    paths: [`~/.${id}`],
    safety,
    regen_cost: 'low',
    description: `${id} description`,
    regen_note: `${id} regen note`,
    cleanup: { kind: 'remove', contents_only: false },
    processes: [],
    docs: null,
    ...overrides,
  }
}

export function finding(id: string, allocated: number, reclaimable = allocated): Finding {
  return {
    rule_id: id,
    status: 'present',
    paths: [`/Users/me/.${id}`],
    size: { allocated, reclaimable, entries: 1, denied: 0 },
  }
}

export function item(id: string, safety: Safety = 'safe', allocated = 1_000_000_000): Item {
  return { rule: rule(id, safety), finding: finding(id, allocated), previous: null }
}
