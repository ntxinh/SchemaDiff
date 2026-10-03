# SchemaDiff Landing Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the SchemaDiff marketing landing page — a Vite/React SPA with a full-viewport R3F 3D scene behind a glassmorphic overlay — entirely on an orphan `gh-pages` branch, deployed by GitHub Pages serving the committed `docs/` build output.

**Architecture:** Fixed `<Canvas>` (TorusKnot + MeshDistortMaterial + drei Float + colored point lights + Stars) at z-0 with pointer-events disabled; a normal-scrolling absolute HTML overlay at z-10 (pointer-events-none, re-enabled per interactive element) renders navbar, hero, bento features, 3-tier OSS pricing, footer. A `useScrollProgress` ref + `useFrame` lerp drives scroll-parallax of the 3D object — no ScrollControls, no state in the render loop.

**Tech Stack:** Vite + React 19 + TypeScript, Tailwind CSS v4 (`@tailwindcss/vite`), `three` + `@react-three/fiber` + `@react-three/drei`, `framer-motion`, `lucide-react`, fontsource Inter + JetBrains Mono.

**Spec:** `docs/superpowers/specs/2026-10-03-landing-page-design.md` (lives on `main`; the site lives on `gh-pages` — read the spec from main's checkout or git history).

## Global Constraints

- Everything for the site happens on an **orphan `gh-pages` branch**. Never switch `main`'s working tree; use a separate worktree (see Task 1).
- `vite.config.ts`: `base: '/SchemaDiff/'`, `build.outDir: 'docs'`, `build.emptyOutDir: true`.
- `.gitignore` covers `node_modules/` and `dist/` only — **`docs/` must NOT be ignored** (it is the deployed artifact).
- Procedural geometry only; no `.gltf`/`.obj`, no image files, no remote font links (fontsource only).
- Canvas: `pointer-events-none`, `dpr: [1, 2]`, background `#050505`. Overlay root: `pointer-events-none`; each link/button gets `pointer-events-auto`.
- Colors: bg `#050505`, cyan `#00F0FF`, purple `#B026FF`, orange `#FF4D4D`.
- Conventional commits.
- Repo constants used in copy: `https://github.com/ntxinh/SchemaDiff`.

## File Structure

```
gh-pages branch root:
  index.html
  package.json  package-lock.json  .gitignore
  vite.config.ts  tsconfig.json  tsconfig.node.json
  src/index.css
  src/main.tsx
  src/App.tsx
  src/vite-env.d.ts
  src/scene/Scene.tsx
  src/hooks/useScrollProgress.ts
  src/components/{Nav,Hero,Features,Pricing,Footer}.tsx
  docs/                     # built output, committed
```

---

### Task 1: Scaffold gh-pages branch + Vite/Tailwind/React project

**Files:**
- Create: worktree `<repo>-ghpages/` on orphan branch `gh-pages`
- Create: `package.json`, `tsconfig.json`, `tsconfig.node.json`, `vite.config.ts`, `index.html`, `src/index.css`, `src/main.tsx`, `src/App.tsx`, `src/vite-env.d.ts`, `.gitignore`

**Interfaces:**
- Produces: the project skeleton every later task builds in. `App.tsx` exports default `App` rendering a placeholder.

- [ ] **Step 1: Create the orphan branch in a separate worktree**

From the main checkout (`/home/exodia/GitRepos/MyGits/SchemaDiff`):

```bash
git checkout --orphan gh-pages
git rm -rf . --quiet
git commit --allow-empty -m "chore: init gh-pages"
git worktree add ../SchemaDiff-ghpages gh-pages
git switch main            # restore main checkout; orphan branch now lives only in the worktree
cd ../SchemaDiff-ghpages
```

All remaining tasks run inside `../SchemaDiff-ghpages`. `main` keeps its files; the worktree holds only the site.

- [ ] **Step 2: Write project files**

`.gitignore`:

```
node_modules/
dist/
```

`package.json` (fill version ranges below at install time — write `"latest"`-compatible carets after `npm install` resolves):

```json
{
  "name": "schemadiff-landing",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview",
    "deploy": "vite build && git add docs && git commit -m 'site: rebuild' || true"
  }
}
```

Then install (this resolves real versions into package.json + lockfile):

```bash
npm install react react-dom
npm install -D vite @vitejs/plugin-react typescript @types/react @types/react-dom tailwindcss @tailwindcss/vite
npm install three @react-three/fiber @react-three/drei framer-motion lucide-react
npm install @types/three -D
npm install @fontsource/inter @fontsource/jetbrains-mono
```

`tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "isolatedModules": true,
    "types": ["vite/client"]
  },
  "include": ["src"]
}
```

`tsconfig.node.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "types": ["node"]
  },
  "include": ["vite.config.ts"]
}
```

(If `@types/node` isn't installed, drop `"types": ["node"]` — `vite.config.ts` here uses no node APIs.)

`vite.config.ts`:

```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  base: '/SchemaDiff/',
  plugins: [react(), tailwindcss()],
  build: { outDir: 'docs', emptyOutDir: true },
})
```

`index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>SchemaDiff — MSSQL schema comparison</title>
    <meta name="description" content="Compare two SQL Server databases. See every object diff. Read-only." />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`src/vite-env.d.ts`:

```ts
/// <reference types="vite/client" />
```

`src/index.css`:

```css
@import "tailwindcss";
@import "@fontsource/inter/400.css";
@import "@fontsource/inter/700.css";
@import "@fontsource/jetbrains-mono/400.css";
@import "@fontsource/jetbrains-mono/500.css";

:root {
  --font-sans: "Inter", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, monospace;
}

html, body, #root { height: 100%; }
body {
  background: #050505;
  color: #fff;
  font-family: var(--font-sans);
  overflow-x: hidden;
}
```

`src/main.tsx`:

```tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './index.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
```

`src/App.tsx` (placeholder — Task 3 replaces it):

```tsx
export default function App() {
  return <main className="text-white p-8">SchemaDiff</main>
}
```

- [ ] **Step 3: Verify the skeleton builds**

Run: `npm run build`
Expected: `tsc` clean, vite outputs `docs/` containing `index.html` + `assets/`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: vite+react+tailwind+r3f project skeleton"
```

---

### Task 2: 3D scene + scroll-progress hook

**Files:**
- Create: `src/hooks/useScrollProgress.ts`
- Create: `src/scene/Scene.tsx`
- Modify: `src/App.tsx` (add `<Scene />` + placeholder sections so scrolling exists)

**Interfaces:**
- Consumes: project skeleton from Task 1.
- Produces:
  - `useScrollProgress(): MutableRefObject<number>` — normalized page scroll 0..1, updated in place (no re-render).
  - `Scene` component (default export) — fixed full-viewport Canvas. `App.tsx` renders `<Scene />` first, then a `relative z-10 pointer-events-none` overlay.

- [ ] **Step 1: Write `src/hooks/useScrollProgress.ts`**

```ts
import { useEffect, useRef } from 'react'
export function useScrollProgress() {
  const ref = useRef(0)
  useEffect(() => {
    const update = () => {
      const max = document.documentElement.scrollHeight - window.innerHeight
      ref.current = max > 0 ? window.scrollY / max : 0
    }
    update()
    window.addEventListener('scroll', update, { passive: true })
    window.addEventListener('resize', update)
    return () => {
      window.removeEventListener('scroll', update)
      window.removeEventListener('resize', update)
    }
  }, [])
  return ref
}
```

- [ ] **Step 2: Write `src/scene/Scene.tsx`**

```tsx
import { Canvas, useFrame } from '@react-three/fiber'
import { Float, MeshDistortMaterial, Stars, TorusKnot } from '@react-three/drei'
import { useRef } from 'react'
import * as THREE from 'three'
import { useScrollProgress } from '../hooks/useScrollProgress'

function Knot() {
  const group = useRef<THREE.Group>(null)
  const scroll = useScrollProgress()

  useFrame((state, delta) => {
    const g = group.current
    if (!g) return
    const p = scroll.current
    const { x, y } = state.pointer
    const damp = THREE.MathUtils.damp

    // mouse parallax
    g.rotation.y = damp(g.rotation.y, x * 0.4, 3, delta)
    g.rotation.x = damp(g.rotation.x, -y * 0.3, 3, delta)
    // scroll: hero right -> features left/back -> pricing receded
    g.position.x = damp(g.position.x, THREE.MathUtils.lerp(1.4, -1.6, p), 3, delta)
    g.position.y = damp(g.position.y, THREE.MathUtils.lerp(0, 0.9, p), 3, delta)
    g.position.z = damp(g.position.z, THREE.MathUtils.lerp(0, -2.2, p), 3, delta)
    const s = THREE.MathUtils.lerp(1, 0.6, p)
    g.scale.setScalar(damp(g.scale.x, s, 3, delta))
  })

  return (
    <group ref={group}>
      <Float speed={1.2} rotationIntensity={0.5} floatIntensity={0.8}>
        <TorusKnot args={[1.1, 0.32, 220, 32]}>
          <MeshDistortMaterial
            color="#8be9fd"
            distort={0.35}
            speed={1.6}
            roughness={0.15}
            metalness={0.6}
            clearcoat={1}
            clearcoatRoughness={0.2}
          />
        </TorusKnot>
      </Float>
    </group>
  )
}

export default function Scene() {
  return (
    <Canvas
      style={{ position: 'fixed', inset: 0, background: '#050505', pointerEvents: 'none' }}
      dpr={[1, 2]}
      camera={{ position: [0, 0, 6], fov: 45 }}
      gl={{ antialias: true, alpha: false }}
    >
      <ambientLight intensity={0.15} />
      <pointLight position={[6, 4, 4]} intensity={60} color="#00F0FF" />
      <pointLight position={[-6, -2, 3]} intensity={50} color="#B026FF" />
      <pointLight position={[0, -5, -4]} intensity={40} color="#FF4D4D" />
      <Stars radius={60} depth={40} count={2000} factor={3} fade speed={0.5} />
      <Knot />
    </Canvas>
  )
}
```

- [ ] **Step 3: Wire Scene into `src/App.tsx` with temporary tall placeholder**

```tsx
import Scene from './scene/Scene'

export default function App() {
  return (
    <>
      <Scene />
      <main className="relative z-10 pointer-events-none">
        <section className="min-h-screen" />
        <section className="min-h-screen" />
        <section className="min-h-screen" />
      </main>
    </>
  )
}
```

- [ ] **Step 4: Verify — dev server smoke**

Run: `npm run dev` (background) → open `http://localhost:5173/SchemaDiff/`
Expected: `#050505` page, torus-knot visible right-of-center, stars behind; scrolling drifts the knot left/back/smaller; mouse move parallaxes. Check in the browser (screenshot), then `npm run build` clean. Kill dev server.

- [ ] **Step 5: Commit**

```bash
git add src/
git commit -m "feat: 3d hero scene with scroll parallax"
```

---

### Task 3: UI overlay — Nav, Hero, Features, Pricing, Footer

**Files:**
- Create: `src/components/Nav.tsx`, `Hero.tsx`, `Features.tsx`, `Pricing.tsx`, `Footer.tsx`
- Modify: `src/App.tsx` (replace placeholder sections with components)

**Interfaces:**
- Consumes: `Scene` stays behind; overlay keeps `pointer-events-none` root.
- Produces: final page. Interactive elements use `pointer-events-auto`.

Copy constants (use verbatim): repo URL `https://github.com/ntxinh/SchemaDiff`; headline `"DIFF TWO DATABASES."` / line 2 `"SHIP WITHOUT SURPRISES."`; subtitle `"Compare two SQL Server databases — tables, views, procs, triggers, types — and see every object's diff. Read-only, always."`

- [ ] **Step 1: `src/components/Nav.tsx`**

```tsx
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
```

- [ ] **Step 2: `src/components/Hero.tsx` — staggered char reveal**

```tsx
import { motion } from 'framer-motion'
import { ArrowRight } from 'lucide-react'

const LINES = ['DIFF TWO DATABASES.', 'SHIP WITHOUT SURPRISES.']

export default function Hero() {
  return (
    <section id="top" className="min-h-screen grid md:grid-cols-2 items-center px-6 md:px-12 max-w-7xl mx-auto">
      <div>
        <h1 className="text-5xl md:text-7xl font-extrabold tracking-tight leading-[1.05]">
          {LINES.map((line) => (
            <span key={line} className="block overflow-hidden">
              {line.split('').map((c, i) => (
                <motion.span
                  key={i}
                  className="inline-block"
                  initial={{ y: '110%' }}
                  animate={{ y: 0 }}
                  transition={{ delay: 0.35 + i * 0.035, duration: 0.55, ease: [0.22, 1, 0.36, 1] }}
                >
                  {c === ' ' ? '\u00A0' : c}
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
          className="pointer-events-auto mt-8 inline-flex items-center gap-2 rounded-lg px-6 py-3 font-mono text-sm border border-[#00F0FF]/60 bg-[#00F0FF]/10 text-[#00F0FF] shadow-[0_0_24px_-6px_#00F0FF] hover:bg-[#00F0FF]/20 transition"
        >
          View on GitHub <ArrowRight className="size-4" />
        </a>
      </div>
      <div /> {/* right column: empty, 3D object shows through */}
    </section>
  )
}
```

- [ ] **Step 3: `src/components/Features.tsx` — bento grid**

Cards (exact copy; icons from lucide-react):

1. `GitCompare` — "Live schema compare" — "Point it at any two SQL Server databases. Source to target, or swap the direction."
2. `Database` — "Every object type" — "Tables, columns, constraints, indexes, views, stored procedures, functions, triggers, user-defined types."
3. `Columns` — "Colored side-by-side diff" — "Git-style add / remove / context rows per object, grouped in a tree with change counts."
4. `FileDown` — "Copy or export everything" — "Copy one object's changes or the whole comparison. Export text, JSON, CSV, or best-effort SQL. Read-only — nothing is ever written." (footer line inside card: mono `SELECT-only catalog reads` with `ShieldCheck` icon.)

```tsx
import { Columns, Database, FileDown, GitCompare, ShieldCheck } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'

const CARDS: { icon: LucideIcon; title: string; body: string; span: string; foot?: string }[] = [
  { icon: GitCompare, title: 'Live schema compare', body: 'Point it at any two SQL Server databases. Source to target, or swap the direction.', span: 'md:col-span-4' },
  { icon: Database, title: 'Every object type', body: 'Tables, columns, constraints, indexes, views, stored procedures, functions, triggers, user-defined types.', span: 'md:col-span-2' },
  { icon: Columns, title: 'Colored side-by-side diff', body: 'Git-style add / remove / context rows per object, grouped in a tree with change counts.', span: 'md:col-span-2' },
  { icon: FileDown, title: 'Copy or export everything', body: 'Copy one object\u2019s changes or the whole comparison. Export text, JSON, CSV, or best-effort SQL.', span: 'md:col-span-4', foot: 'Read-only \u2014 nothing is ever written' },
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
```

- [ ] **Step 4: `src/components/Pricing.tsx` — real OSS tiers, middle glows**

Tiers (exact copy):

1. "Open Source" — `$0` — "Free forever." — bullets: "Full tool, no limits", "MIT licensed", "Runs entirely offline" — CTA "Clone the repo".
2. "Sponsor" — `$x` — "Fuel development." — bullets: "GitHub Sponsors", "Vote with issues", "Shape the roadmap" — CTA "Sponsor on GitHub". **Featured/glowing card.**
3. "Teams" — `YOSS` → use `$0` — "Roll it out at work." — bullets: "Docs in the repo", "Issues get priority", "Self-contained binary" — CTA "Open an issue".

```tsx
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
```

- [ ] **Step 5: `src/components/Footer.tsx`**

```tsx
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
```

- [ ] **Step 6: Final `src/App.tsx`**

```tsx
import Scene from './scene/Scene'
import Nav from './components/Nav'
import Hero from './components/Hero'
import Features from './components/Features'
import Pricing from './components/Pricing'
import Footer from './components/Footer'

export default function App() {
  return (
    <>
      <Scene />
      <Nav />
      <main className="relative z-10 pointer-events-none">
        <Hero />
        <Features />
        <Pricing />
        <Footer />
      </main>
    </>
  )
}
```

- [ ] **Step 7: Verify visually**

Run `npm run dev`, open `http://localhost:5173/SchemaDiff/` in the browser tool:
- hero: headline chars stagger in, knot sits in right column, parallax on mouse
- scroll: knot drifts left/back/dimmer; bento cards glassy with scene visible through
- pricing: middle card glows purple over the radial gradient
- footer thin top border, mono links
- responsive: shrink viewport ~390px — single column, knot still centered-ish
Then `npm run build` — must be clean. Kill dev server.

- [ ] **Step 8: Commit**

```bash
git add src/
git commit -m "feat: glassmorphic overlay sections"
```

---

### Task 4: Build, commit docs/, enable Pages, verify live

**Files:**
- Create: `docs/` (build output, committed)
- Modify: `package.json` deploy script if needed

**Interfaces:**
- Consumes: finished site.
- Produces: live URL `https://ntxinh.github.io/SchemaDiff/`.

- [ ] **Step 1: Build + commit output**

```bash
npm run build          # outputs docs/
git add docs
git commit -m "site: production build"
```

- [ ] **Step 2: Push branch and enable Pages**

```bash
git push -u origin gh-pages
gh api repos/ntxinh/SchemaDiff/pages -X POST \
  -f "source[branch]=gh-pages" -f "source[path]=/docs"
```

Expected: API returns the pages object with `"html_url": "https://ntxinh.github.io/SchemaDiff/"`. If POST fails because Pages was enabled concurrently, GET the pages object and confirm branch/path, else `gh api repos/ntxinh/SchemaDiff/pages -X PUT` to update source.

- [ ] **Step 3: Verify live**

Wait ~60s for the first Pages build, then:

```bash
curl -s -o /dev/null -w "%{http_code}" https://ntxinh.github.io/SchemaDiff/
```

Expected: `200`. Then open the URL in the browser tool and screenshot — the 3D hero, glass nav, and sections render identically to local. If assets 404, check `base` and that `docs/` was committed with `assets/`.

- [ ] **Step 4: Push the spec/plan commits on main**

```bash
cd ../SchemaDiff   # main checkout
git push origin main
```

- [ ] **Step 5: Cleanup worktree (optional)**

Keep `../SchemaDiff-ghpages` for future site edits (recommended — it IS the site's home), or remove with `git worktree remove ../SchemaDiff-ghpages`.

---

## Self-Review

- **Spec coverage:** orphan gh-pages branch ✓ T1; docs/ output + base ✓ T1/T4; Canvas/knot/lights/stars ✓ T2; scroll parallax ✓ T2; nav/hero/features/pricing/footer ✓ T3; responsive ✓ T3 step 7; Pages enable + live verify ✓ T4. Fonts via fontsource ✓ T1. No placeholders/images ✓ (none anywhere).
- **Placeholders:** all steps carry real code/copy; no TBD.
- **Type consistency:** `useScrollProgress` returns `MutableRefObject<number>`, consumed as `scroll.current` ✓; `Scene` default export matches App import ✓; card/tier shapes self-consistent ✓.
- **Canvas props:** single `style` prop on `<Canvas>`; no duplicates, no className conflicts.
