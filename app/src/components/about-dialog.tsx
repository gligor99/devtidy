import { useQuery } from '@tanstack/react-query'
import { getVersion } from '@tauri-apps/api/app'
import { openUrl } from '@tauri-apps/plugin-opener'
import { EyeOffIcon, GaugeIcon, ShieldCheckIcon, WrenchIcon } from 'lucide-react'
import { LogoMark } from '@/components/logo'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { BRAND } from '@/lib/brand'

const PRINCIPLES = [
  { icon: ShieldCheckIcon, text: "Never deletes outside a rule's paths, never follows symlinks." },
  { icon: GaugeIcon, text: 'Honest sizes: allocated blocks, hard links counted once.' },
  { icon: WrenchIcon, text: "Uses each tool's own cleanup command when it has one." },
  { icon: EyeOffIcon, text: 'No telemetry. Nothing leaves your machine.' },
]

export function useAppVersion() {
  return useQuery({ queryKey: ['version'], queryFn: getVersion, staleTime: Infinity }).data
}

export function AboutDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const version = useAppVersion()
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-sm">
        <div className="flex flex-col items-center gap-2 pt-2 text-center">
          <LogoMark size={72} className="drop-shadow-md" />
          <DialogTitle className="text-lg">{BRAND.name}</DialogTitle>
          {version && <p className="text-muted-foreground -mt-1 text-xs">Version {version}</p>}
          <DialogDescription>{BRAND.tagline}</DialogDescription>
        </div>
        <ul className="flex flex-col gap-2.5 py-2">
          {PRINCIPLES.map(({ icon: Icon, text }) => (
            <li key={text} className="flex items-start gap-2.5 text-[13px]">
              <Icon className="text-safe mt-0.5 size-4 shrink-0" />
              <span>{text}</span>
            </li>
          ))}
        </ul>
        <div className="flex justify-center gap-2">
          <Button variant="outline" size="sm" onClick={() => void openUrl(BRAND.repo)}>
            GitHub
          </Button>
          <Button variant="outline" size="sm" onClick={() => void openUrl(BRAND.issues)}>
            Report an issue
          </Button>
        </div>
        <p className="text-muted-foreground text-center text-[11px]">
          Open source · {BRAND.license}
        </p>
      </DialogContent>
    </Dialog>
  )
}
