// @ts-check
import starlight from '@astrojs/starlight'
import { defineConfig } from 'astro/config'

const site = 'https://gligor99.github.io'
const base = '/devtidy'

// https://astro.build/config
export default defineConfig({
  site,
  base,
  trailingSlash: 'always',
  integrations: [
    starlight({
      title: 'DevTidy',
      description: 'See and safely reclaim disk space used by developer tooling.',
      logo: { src: './src/assets/icon.svg', alt: 'DevTidy' },
      favicon: '/favicon.svg',
      customCss: ['./src/styles/theme.css'],
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/gligor99/devtidy' }],
      editLink: { baseUrl: 'https://github.com/gligor99/devtidy/edit/main/website/' },
      lastUpdated: true,
      head: [
        { tag: 'meta', attrs: { property: 'og:image', content: `${site}${base}/og.png` } },
        { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
        { tag: 'meta', attrs: { name: 'theme-color', content: '#0B1412' } },
        { tag: 'link', attrs: { rel: 'apple-touch-icon', href: `${base}/apple-touch-icon.png` } },
      ],
      sidebar: [
        {
          label: 'Start here',
          items: [
            { label: 'Introduction', slug: 'docs' },
            { label: 'Install', slug: 'docs/install' },
            { label: 'Desktop app', slug: 'docs/app' },
          ],
        },
        {
          label: 'CLI',
          items: [
            { label: 'Overview', slug: 'docs/cli' },
            { label: 'scan', slug: 'docs/cli/scan' },
            { label: 'explain', slug: 'docs/cli/explain' },
            { label: 'clean', slug: 'docs/cli/clean' },
            { label: 'history & rules', slug: 'docs/cli/history' },
            { label: 'JSON output', slug: 'docs/cli/json' },
          ],
        },
        {
          label: 'Concepts',
          items: [
            { label: 'Safety model', slug: 'docs/safety' },
            { label: 'How sizes are measured', slug: 'docs/sizes' },
            { label: 'Privacy', slug: 'docs/privacy' },
          ],
        },
        {
          label: 'Rules',
          items: [
            { label: 'Built-in rules', slug: 'docs/rules' },
            { label: 'Custom rules', slug: 'docs/rules/custom' },
          ],
        },
        { label: 'Changelog', slug: 'docs/changelog' },
      ],
    }),
  ],
})
