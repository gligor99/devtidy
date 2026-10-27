import { useId } from 'react'

/** The app icon as an inline mark (simplified from assets/icon.svg: no margin or shadows). */
export function LogoMark({ size = 28, className }: { size?: number; className?: string }) {
  const id = useId()
  return (
    <svg width={size} height={size} viewBox="100 100 824 824" className={className} aria-hidden="true">
      <defs>
        <linearGradient id={`${id}-bg`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#34D3B4" />
          <stop offset="0.55" stopColor="#119C86" />
          <stop offset="1" stopColor="#0B5F55" />
        </linearGradient>
      </defs>
      <rect x="100" y="100" width="824" height="824" rx="186" fill={`url(#${id}-bg)`} />
      <g fill="none" stroke="#fff" strokeWidth="58" strokeLinecap="round" strokeLinejoin="round">
        <path d="M 426 330 C 356 330 352 368 352 424 L 352 462 C 352 496 336 512 296 512 C 336 512 352 528 352 562 L 352 600 C 352 656 356 694 426 694" />
        <path d="M 598 330 C 668 330 672 368 672 424 L 672 462 C 672 496 688 512 728 512 C 688 512 672 528 672 562 L 672 600 C 672 656 668 694 598 694" />
      </g>
      <path fill="#FCD34D" d="M 512 400 Q 523 501 614 512 Q 523 523 512 624 Q 501 523 410 512 Q 501 501 512 400 Z" />
    </svg>
  )
}
