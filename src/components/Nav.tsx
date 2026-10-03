import { Rocket } from 'lucide-react'

export default function Nav() {
  return (
    <header className="fixed top-0 inset-x-0 z-20 backdrop-blur-xl bg-black/30 border-b border-white/10 pointer-events-auto">
      <nav className="mx-auto max-w-6xl px-6 h-14 flex items-center justify-between">
        <a href="#top" className="font-bold text-lg tracking-tight">SchemaDiff</a>
        <a
          href="https://github.com/ntxinh/SchemaDiff"
          className="inline-flex items-center gap-2 rounded-md px-4 py-1.5 text-sm font-medium border border-[#00F0FF]/50 text-[#00F0FF] shadow-[0_0_18px_-4px_#00F0FF] hover:bg-[#00F0FF]/10 transition"
        >
          <Rocket className="size-4" /> Build launch plan
        </a>
      </nav>
    </header>
  )
}
