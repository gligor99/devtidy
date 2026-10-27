import { Channel } from '@tauri-apps/api/core'
import { commands, type ScanEvent } from '@/bindings'

type Result<T, E> = { status: 'ok'; data: T } | { status: 'error'; error: E }

/** Turns a tauri-specta result into a value or a thrown Error, for TanStack Query. */
export async function unwrap<T>(promise: Promise<Result<T, string>>): Promise<T> {
  const result = await promise
  if (result.status === 'error') throw new Error(result.error)
  return result.data
}

export function scan(onEvent: (event: ScanEvent) => void) {
  const channel = new Channel<ScanEvent>()
  channel.onmessage = onEvent
  return unwrap(commands.scan(channel))
}

export const planClean = (ids: string[]) => unwrap(commands.planClean(ids))
export const clean = (ids: string[]) => unwrap(commands.clean(ids))
export const getHistory = (limit: number) => unwrap(commands.getHistory(limit))
export const getRules = () => commands.getRules()
export const getDisk = () => commands.getDisk()
