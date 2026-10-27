// Facts about the project, read from the repository at build time so the
// site can never drift from the code it documents.
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { parse } from 'smol-toml'

// Astro bundles this module at build time, so resolve from the website folder.
const repoRoot = resolve(process.cwd(), '..') + '/'
const read = (path: string) => readFileSync(repoRoot + path, 'utf8')

export const REPO = 'https://github.com/gligor99/devtidy'

export const VERSION: string = JSON.parse(read('app/package.json')).version

const releaseFile = (name: string) => `${REPO}/releases/download/v${VERSION}/${name}`

export const DOWNLOADS = {
  dmg: releaseFile(`DevTidy_${VERSION}_universal.dmg`),
  cliMac: releaseFile(`devtidy-${VERSION}-macos-universal.tar.gz`),
  cliLinux: releaseFile(`devtidy-${VERSION}-linux-x86_64.tar.gz`),
  checksums: releaseFile('SHA256SUMS'),
  releases: `${REPO}/releases/latest`,
}

export type Safety = 'safe' | 'caution' | 'dangerous'

export type Rule = {
  id: string
  name: string
  ecosystem: string
  platforms?: string[]
  paths: string[]
  safety: Safety
  regen_cost: 'low' | 'medium' | 'high'
  description: string
  regen_note: string
  cleanup: { kind: 'remove'; contents_only?: boolean } | { kind: 'command'; argv: string[]; fallback_remove?: boolean }
  processes?: string[]
  docs?: string
}

export const ECOSYSTEM_LABELS: Record<string, string> = {
  ios: 'iOS / Xcode',
  android: 'Android',
  javascript: 'JavaScript',
  flutter: 'Flutter',
  rust: 'Rust',
  python: 'Python',
  docker: 'Docker',
  system: 'System',
}

const rulesDir = 'crates/rules/rules/'

/** Every built-in rule, in the order `crates/rules` embeds them (a missing file fails the build). */
export const RULES: Rule[] = ['ios.toml', 'android.toml', 'javascript.toml', 'other.toml'].flatMap((file) => (parse(read(rulesDir + file)) as unknown as { rule: Rule[] }).rule)

export function describeCleanup(rule: Rule): string {
  const removal = { safe: 'Deleted permanently', caution: 'Moved to the Trash', dangerous: 'Moved to the Trash, only on its own' }[
    rule.safety
  ]
  const c = rule.cleanup
  if (c.kind === 'command') {
    const run = `Runs \`${c.argv.join(' ')}\``
    return c.fallback_remove ? `${run}, otherwise ${removal.toLowerCase()}` : run
  }
  return c.contents_only ? `${removal} (contents only, the folder is kept)` : removal
}

export const CHANGELOG = read('CHANGELOG.md')
