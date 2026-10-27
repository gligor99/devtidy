// Dev-only demo: renders the real app UI against a mocked backend, for
// screenshots (README, website, Ko-fi). Open /demo.html in `pnpm dev`.
//
// Query params drive the UI after it renders:
//   ?focus=<rule id>    open the details panel for an item
//   &select=<id,id>     tick items
//   &dialog=1           open the cleanup review dialog
//   &view=history       show the history view
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { mockIPC } from '@tauri-apps/api/mocks'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from '@/App'
import type { Finding, Plan, Rule_Serialize as Rule } from '@/bindings'
import { Toaster } from '@/components/ui/sonner'
import '@/index.css'
import rulesJson from './rules.json'

const rules = rulesJson as Rule[]
const GB = 1_000_000_000
const now = Math.floor(Date.now() / 1000)
const DAY = 86_400

// Sizes from a real developer machine, rounded.
const sizes: Record<string, [allocated: number, reclaimable?: number, previous?: number]> = {
  'android.system-images': [31.5 * GB],
  'android.avd': [20.2 * GB],
  'simulator.devices': [18.3 * GB],
  'xcode.derived-data': [18.2 * GB, undefined, 15.1 * GB],
  'gradle.caches': [12.1 * GB, undefined, 8.1 * GB],
  'npm.cache': [9.2 * GB],
  'pnpm.store': [8.4 * GB, 1.3 * GB],
  'yarn.cache': [6.3 * GB],
  'cocoapods.cache': [1.7 * GB],
  'bun.cache': [1.6 * GB],
  'xcode.previews': [1.4 * GB],
  'cargo.registry': [0.47 * GB],
  'homebrew.cache': [0.38 * GB],
  'pub.cache': [0.32 * GB],
  'gradle.wrapper-dists': [0.31 * GB],
}

const home = '/Users/you'
const findings: Finding[] = rules.map((rule) => {
  const size = sizes[rule.id]
  return {
    rule_id: rule.id,
    status: size ? 'present' : 'missing',
    paths: size ? [rule.paths[0]!.replace('~', home).replace(/\*\.avd$/, 'Pixel_8_API_35.avd').replaceAll('*', 'android-36')] : [],
    size: { allocated: size?.[0] ?? 0, reclaimable: size?.[1] ?? size?.[0] ?? 0, entries: 1, denied: 0 },
  }
})
const previous = Object.fromEntries(
  Object.entries(sizes)
    .filter(([, s]) => s[2] !== undefined)
    .map(([id, s]) => [id, { at: now - 6 * DAY, allocated: s[2]! }]),
)

function plan(ids: string[]): Plan {
  const actions = ids.map((id) => {
    const rule = rules.find((r) => r.id === id)!
    const finding = findings.find((f) => f.rule_id === id)!
    const method =
      rule.cleanup.kind === 'command'
        ? { kind: 'command' as const, argv: rule.cleanup.argv }
        : { kind: rule.safety === 'safe' ? ('delete' as const) : ('trash' as const) }
    return {
      rule_id: id,
      name: rule.name,
      safety: rule.safety,
      method,
      roots: finding.paths,
      targets: finding.paths,
      estimated: finding.size.reclaimable,
      blocked_by: id === 'simulator.caches' ? ['Simulator'] : [],
    }
  })
  return { actions, skipped: [] }
}

mockIPC((cmd, args) => {
  const a = (args ?? {}) as Record<string, unknown>
  switch (cmd) {
    case 'get_rules':
      return rules
    case 'get_disk':
      return { total: 494 * GB, available: 48 * GB }
    case 'scan':
      return { scanned_at: now, disk: { total: 494 * GB, available: 48 * GB }, findings, previous }
    case 'plan_clean':
      return plan(a.ids as string[])
    case 'get_history':
      return [
        {
          id: 2,
          at: now - 2 * 3600,
          freed: 30.1 * GB,
          trashed: 0,
          actions: [
            { rule_id: 'xcode.derived-data', method: 'delete', estimated: 18.2 * GB, outcome: 'done', detail: null },
            { rule_id: 'gradle.caches', method: 'delete', estimated: 12.1 * GB, outcome: 'done', detail: null },
          ],
        },
        {
          id: 1,
          at: now - 9 * DAY,
          freed: 4.6 * GB,
          trashed: 1.3 * GB,
          actions: [
            { rule_id: 'yarn.cache', method: 'delete', estimated: 2.1 * GB, outcome: 'done', detail: null },
            { rule_id: 'pnpm.store', method: 'trash', estimated: 1.3 * GB, outcome: 'done', detail: null },
            { rule_id: 'xcode.previews', method: 'delete', estimated: 1.4 * GB, outcome: 'done', detail: null },
            { rule_id: 'simulator.caches', method: 'delete', estimated: 0, outcome: 'blocked', detail: 'Simulator' },
          ],
        },
      ]
    case 'plugin:app|version':
      return '0.2.0'
    default:
      return null
  }
})

const queryClient = new QueryClient({ defaultOptions: { queries: { refetchOnWindowFocus: false, retry: false } } })

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
      <Toaster position="bottom-right" />
    </QueryClientProvider>
  </StrictMode>,
)

// Drive the UI from query params once the scan result has rendered.
const params = new URLSearchParams(location.search)
const click = (el: Element | null | undefined) => (el as HTMLElement | null)?.click()
const byText = (selector: string, text: string) =>
  [...document.querySelectorAll(selector)].find((el) => el.textContent?.includes(text))

setTimeout(() => {
  if (params.get('view') === 'history') click(byText('nav button', 'History'))
  for (const id of params.get('select')?.split(',') ?? []) {
    const name = rules.find((r) => r.id === id)?.name
    click(document.querySelector(`[aria-label="Select ${name}"]`))
  }
  const focus = rules.find((r) => r.id === params.get('focus'))
  if (focus) click(byText('[role="option"]', focus.name))
  if (params.get('dialog')) setTimeout(() => click(byText('footer button', 'Clean')), 100)
}, 300)
