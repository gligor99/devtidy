import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { Plan, Report_Serialize as Report } from '@/bindings'
import * as api from '@/lib/api'
import { CleanDialog } from './clean-dialog'

vi.mock('@/lib/api', () => ({ planClean: vi.fn(), clean: vi.fn() }))

const action = (id: string, safety: Plan['actions'][number]['safety'], blocked: string[] = []) => ({
  rule_id: id,
  name: `${id} name`,
  safety,
  method: { kind: 'delete' as const },
  roots: [],
  targets: [],
  estimated: 2_000_000_000,
  blocked_by: blocked,
})

const report: Report = {
  results: [{ rule_id: 'gradle.caches', method: { kind: 'delete' }, estimated: 2e9, outcome: { result: 'done' } }],
  freed: 1_900_000_000,
  trashed: 0,
}

function renderDialog(ids: string[]) {
  const onCleaned = vi.fn()
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <CleanDialog ids={ids} open onOpenChange={() => {}} onCleaned={onCleaned} />
    </QueryClientProvider>,
  )
  return { onCleaned }
}

beforeEach(() => vi.resetAllMocks())

it('shows the plan and only cleans after confirmation', async () => {
  vi.mocked(api.planClean).mockResolvedValue({ actions: [action('gradle.caches', 'safe')], skipped: [] })
  vi.mocked(api.clean).mockResolvedValue(report)
  const { onCleaned } = renderDialog(['gradle.caches'])

  expect(await screen.findByText('gradle.caches name')).toBeInTheDocument()
  expect(api.clean).not.toHaveBeenCalled()

  await userEvent.click(screen.getByRole('button', { name: 'Clean 2.0 GB' }))

  expect(await screen.findByText(/Freed/)).toHaveTextContent('Freed 1.9 GB (measured on disk).')
  expect(api.clean).toHaveBeenCalledWith(['gradle.caches'])
  expect(onCleaned).toHaveBeenCalledWith(report)
})

it('requires typing the id for a dangerous item', async () => {
  vi.mocked(api.planClean).mockResolvedValue({ actions: [action('xcode.archives', 'dangerous')], skipped: [] })
  renderDialog(['xcode.archives'])

  const button = await screen.findByRole('button', { name: 'Clean 2.0 GB' })
  expect(button).toBeDisabled()

  await userEvent.type(screen.getByLabelText('Confirm id'), 'xcode.archive')
  expect(button).toBeDisabled()
  await userEvent.type(screen.getByLabelText('Confirm id'), 's')
  expect(button).toBeEnabled()
})

it('cannot clean when everything is blocked by a running app', async () => {
  vi.mocked(api.planClean).mockResolvedValue({ actions: [action('xcode.derived-data', 'safe', ['Xcode'])], skipped: [] })
  renderDialog(['xcode.derived-data'])

  expect(await screen.findByText('Skipped: quit Xcode first')).toBeInTheDocument()
  expect(screen.getByRole('button', { name: /Clean/ })).toBeDisabled()
})
