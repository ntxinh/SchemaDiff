import { Columns, Database, FileDown, GitCompare, ShieldCheck } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'

const CARDS: { icon: LucideIcon; title: string; body: string; span: string; foot?: string }[] = [
  { icon: GitCompare, title: 'Live schema compare', body: 'Point it at any two SQL Server databases. Source to target, or swap the direction.', span: 'md:col-span-4' },
  { icon: Database, title: 'Every object type', body: 'Tables, columns, constraints, indexes, views, stored procedures, functions, triggers, user-defined types.', span: 'md:col-span-2' },
  { icon: Columns, title: 'Colored side-by-side diff', body: 'Git-style add / remove / context rows per object, grouped in a tree with change counts.', span: 'md:col-span-2' },
  { icon: FileDown, title: 'Copy or export everything', body: 'Copy one object’s changes or the whole comparison. Export text, JSON, CSV, or best-effort SQL.', span: 'md:col-span-4', foot: 'Read-only — nothing is ever written' },
]

export default function Features() {
  return (
    <section className="px-6 md:px-12 py-28 max-w-6xl mx-auto">
      <h2 className="font-mono text-sm text-[#00F0FF] tracking-widest mb-10">// WHAT IT DOES</h2>
      <div className="grid md:grid-cols-6 gap-4">
        {CARDS.map(({ icon: Icon, title, body, span, foot }) => (
          <div key={title} className={`${span} rounded-2xl border border-white/10 bg-white/5 backdrop-blur-xl p-6`}>
            <Icon className="size-6 text-[#00F0FF]" />
            <h3 className="mt-4 font-semibold text-lg">{title}</h3>
            <p className="mt-2 text-sm text-white/60">{body}</p>
            {foot && (
              <p className="mt-4 font-mono text-xs text-white/40 flex items-center gap-2">
                <ShieldCheck className="size-4" /> {foot}
              </p>
            )}
          </div>
        ))}
      </div>
    </section>
  )
}
