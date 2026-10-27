import type { Ecosystem } from '@/bindings'

const UNITS = ['B', 'kB', 'MB', 'GB', 'TB']

/** SI units (1 GB = 10^9 bytes), matching Finder and the CLI. */
export function formatBytes(bytes: number): string {
  let value = bytes
  let unit = 0
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000
    unit += 1
  }
  return unit === 0 ? `${bytes} B` : `${value.toFixed(1)} ${UNITS[unit]}`
}

/** Signed size change, or null when it is too small to mention. */
export function formatDelta(bytes: number): string | null {
  if (Math.abs(bytes) < 1_000_000) return null
  return `${bytes > 0 ? '+' : '−'}${formatBytes(Math.abs(bytes))}`
}

export function formatAgo(unixSeconds: number, now = Date.now() / 1000): string {
  const secs = Math.max(0, Math.floor(now - unixSeconds))
  if (secs < 60) return 'just now'
  if (secs < 3600) return `${Math.floor(secs / 60)} min ago`
  if (secs < 86_400) return `${Math.floor(secs / 3600)} h ago`
  const days = Math.floor(secs / 86_400)
  return days === 1 ? 'yesterday' : `${days} days ago`
}

export const ECOSYSTEM_LABELS = {
  ios: 'iOS / Xcode',
  android: 'Android',
  javascript: 'JavaScript',
  flutter: 'Flutter',
  rust: 'Rust',
  python: 'Python',
  docker: 'Docker',
  system: 'System',
} satisfies Record<Ecosystem, string>
