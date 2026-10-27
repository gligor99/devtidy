import { useMutation, useQuery } from '@tanstack/react-query'
import { useCallback, useEffect, useRef, useState } from 'react'
import type { Finding, ScanResult_Serialize as ScanResult } from '@/bindings'
import * as api from '@/lib/api'

export type ScanState = {
  /** Findings so far; filled live while a scan runs. */
  findings: Finding[]
  result: ScanResult | null
  progress: { done: number; total: number } | null
  scanning: boolean
  error: Error | null
  rescan: () => void
}

/** Runs a scan on mount and on demand, streaming findings as they arrive. */
export function useScan(): ScanState {
  const [findings, setFindings] = useState<Finding[]>([])
  const [progress, setProgress] = useState<ScanState['progress']>(null)

  const mutation = useMutation({
    mutationKey: ['scan'],
    mutationFn: () => {
      setProgress({ done: 0, total: 0 })
      return api.scan((event) => {
        if (event.type === 'started') {
          setProgress({ done: 0, total: event.total })
        } else {
          setFindings((prev) => [...prev.filter((f) => f.rule_id !== event.finding.rule_id), event.finding])
          setProgress((p) => (p ? { ...p, done: p.done + 1 } : p))
        }
      })
    },
    onSuccess: (result) => setFindings(result.findings),
    onSettled: () => setProgress(null),
  })

  const { mutate } = mutation
  const rescan = useCallback(() => mutate(), [mutate])
  // Refs survive StrictMode's double effect run, so only one scan starts.
  const started = useRef(false)
  useEffect(() => {
    if (started.current) return
    started.current = true
    rescan()
  }, [rescan])

  return {
    findings,
    result: mutation.data ?? null,
    progress,
    scanning: mutation.isPending,
    error: mutation.error,
    rescan,
  }
}

export function useRules() {
  return useQuery({ queryKey: ['rules'], queryFn: api.getRules, staleTime: Infinity })
}
