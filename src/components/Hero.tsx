import { motion } from 'framer-motion'
import { ArrowRight } from 'lucide-react'

const LINES = ['DIFF TWO DATABASES.', 'SHIP WITHOUT SURPRISES.']

export default function Hero() {
  return (
    <section id="top" className="min-h-screen grid md:grid-cols-2 items-center px-6 md:px-12 max-w-7xl mx-auto">
      <div>
        <h1 aria-label="Diff two databases. Ship without surprises." className="text-5xl md:text-7xl font-extrabold tracking-tight leading-[1.05]">
          {LINES.map((line) => (
            <span key={line} aria-hidden="true" className="block overflow-hidden">
              {line.split('').map((c, i) => (
                <motion.span
                  key={i}
                  className="inline-block"
                  initial={{ y: '110%' }}
                  animate={{ y: 0 }}
                  transition={{ delay: 0.35 + i * 0.035, duration: 0.55, ease: [0.22, 1, 0.36, 1] }}
                >
                  {c === ' ' ? ' ' : c}
                </motion.span>
              ))}
            </span>
          ))}
        </h1>
        <p className="mt-6 font-mono text-sm md:text-base text-white/60 max-w-md">
          Compare two SQL Server databases — tables, views, procs, triggers,
          types — and see every object's diff. Read-only, always.
        </p>
        <a
          href="https://github.com/ntxinh/SchemaDiff"
          className="mt-8 inline-flex items-center gap-2 rounded-lg px-6 py-3 font-mono text-sm border border-[#00F0FF]/60 bg-[#00F0FF]/10 text-[#00F0FF] shadow-[0_0_24px_-6px_#00F0FF] hover:bg-[#00F0FF]/20 transition"
        >
          View on GitHub <ArrowRight className="size-4" />
        </a>
      </div>
      <div /> {/* right column: empty, 3D object shows through */}
    </section>
  )
}
