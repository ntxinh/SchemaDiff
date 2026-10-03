import { Check } from 'lucide-react'

const TIERS = [
  { name: 'Open Source', price: '$0', tag: 'Free forever.', perks: ['Full tool, no limits', 'MIT licensed', 'Runs entirely offline'], cta: 'Clone the repo', href: 'https://github.com/ntxinh/SchemaDiff', featured: false },
  { name: 'Sponsor', price: '$x', tag: 'Fuel development.', perks: ['GitHub Sponsors', 'Vote with issues', 'Shape the roadmap'], cta: 'Sponsor on GitHub', href: 'https://github.com/sponsors/ntxinh', featured: true },
  { name: 'Teams', price: '$0', tag: 'Roll it out at work.', perks: ['Docs in the repo', 'Issues get priority', 'Self-contained binary'], cta: 'Open an issue', href: 'https://github.com/ntxinh/SchemaDiff/issues', featured: false },
]

export default function Pricing() {
  return (
    <section className="relative px-6 md:px-12 py-28 max-w-6xl mx-auto">
      <div className="absolute inset-0 -z-10 bg-[radial-gradient(ellipse_at_center,rgba(176,38,255,0.18),transparent_60%)]" />
      <h2 className="font-mono text-sm text-[#B026FF] tracking-widest mb-10">// WHAT IT COSTS</h2>
      <div className="grid md:grid-cols-3 gap-4">
        {TIERS.map((t) => (
          <div
            key={t.name}
            className={`rounded-2xl border p-6 backdrop-blur-xl ${
              t.featured
                ? 'border-[#B026FF]/50 bg-white/10 shadow-[0_0_50px_-12px_#B026FF]'
                : 'border-white/10 bg-white/5'
            }`}
          >
            <h3 className="font-semibold text-lg">{t.name}</h3>
            <p className="mt-2 text-4xl font-extrabold">{t.price}</p>
            <p className="mt-1 font-mono text-xs text-white/50">{t.tag}</p>
            <ul className="mt-5 space-y-2 text-sm text-white/70">
              {t.perks.map((p) => (
                <li key={p} className="flex items-center gap-2">
                  <Check className="size-4 text-[#00F0FF]" /> {p}
                </li>
              ))}
            </ul>
            <a href={t.href} className="pointer-events-auto mt-6 block text-center rounded-lg border border-white/15 px-4 py-2 text-sm font-mono hover:bg-white/10 transition">
              {t.cta}
            </a>
          </div>
        ))}
      </div>
    </section>
  )
}
