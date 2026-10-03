export default function Footer() {
  return (
    <footer className="border-t border-white/10 font-mono text-xs text-white/40">
      <div className="max-w-6xl mx-auto px-6 py-8 flex flex-col md:flex-row items-center justify-between gap-3">
        <span>© 2026 ntxinh — MIT</span>
        <a href="https://github.com/ntxinh/SchemaDiff" className="pointer-events-auto hover:text-white/70 transition">
          github.com/ntxinh/SchemaDiff
        </a>
      </div>
    </footer>
  )
}
