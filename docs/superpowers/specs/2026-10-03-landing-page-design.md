# SchemaDiff Landing Page — Design Spec

## Goal

A single-page, static-hosted product landing page for SchemaDiff on GitHub
Pages at `https://ntxinh.github.io/SchemaDiff/`, with a real-time 3D hero
object behind a glassmorphic UI overlay. No server-side logic.

## Hosting & deployment

- New **orphan branch `gh-pages`** holds the entire Vite project at branch
  root (`index.html`, `src/`, `package.json`, `vite.config.ts`, …).
- `vite build` outputs to **`docs/`** (not `dist/`). GitHub Pages is
  configured to serve from `gh-pages` branch, `/docs` folder — one commit
  carries source and built site; a push is a deploy.
- `vite.config.ts` sets `base: '/SchemaDiff/'`.
- Republish = `npm run build` + commit `docs/` (wrapped in an npm script).
- Pages is currently disabled on the repo; enable via
  `gh api repos/ntxinh/SchemaDiff/pages -X POST` after the branch exists.

## Stack

Vite + React + TypeScript. Tailwind CSS v4 (`@tailwindcss/vite` plugin).
`three`, `@react-three/fiber`, `@react-three/drei` for the scene.
`framer-motion` for DOM animation. `lucide-react` for icons.
Fonts: Inter + JetBrains Mono via fontsource packages (fully self-hosted).
No routing, no state library, no images, no external 3D assets.

## Scene (`src/scene/Scene.tsx`)

- One `<Canvas>`: `position: fixed`, full viewport, behind all overlay
  content; `dpr` capped at `[1, 2]`; background `#050505`.
- Central object: `TorusKnot` (~r1.1, tube 0.32, 220×32 segs) with
  `MeshDistortMaterial` (distort ~0.35, clearcoat for glassy feel) inside
  drei `<Float>` for slow rotation/drift.
- Lighting: `ambientLight` intensity ~0.15 + three `pointLight`s —
  cyan `#00F0FF`, purple `#B026FF`, orange `#FF4D4D` — positioned to
  rim-light the geometry. `<Stars>` for the deep-space backdrop.
- Mouse parallax: `useFrame` lerps the object's group rotation/position
  toward the pointer (ref-based, no React state).

## Scroll behavior

- `useScrollProgress()` hook: `window.scroll` listener writing progress
  (0..1) to a ref; `useFrame` consumes it.
- Knot eases right in hero, drifts left/back and slightly smaller through
  features, dims/recedes under pricing. No `ScrollControls` — the overlay
  owns native scrolling.

## Overlay (absolute/fixed HTML above the canvas)

- **Navbar** — fixed top, `backdrop-blur-xl bg-black/30 border-b
  border-white/10`. Left: "SchemaDiff" (Inter, bold, white). Right: CTA
  "Build launch plan" → GitHub repo link, subtle glowing cyan border.
- **Hero** — first viewport, two-column grid. Left: kinetic headline via
  Framer Motion staggered character reveal — "Diff two databases. Ship
  without surprises." (≤8 words) — JetBrains Mono subtitle, holographic
  "View on GitHub" button. Right column stays empty for the 3D object.
- **Features** — asymmetric bento grid (`grid-cols-6`, spans 4/2/2/4),
  semi-transparent glass cards over the moving scene. Four cards, copy
  from the real feature set:
  1. Live schema compare — two MSSQL databases, source ↔ target
     (GitCompare icon).
  2. Every object type — tables, views, procs, functions, triggers, UDTs
     (Database).
  3. Colored side-by-side diff — add/remove/context rows (Diff icon /
     Columns).
  4. Export — copy changes or export text/JSON/CSV/SQL; read-only,
     nothing written (FileDown / ShieldCheck).
- **Pricing (real OSS tiers)** — 3 glass cards, middle one glowing via
  underlying radial gradient:
  1. **Open Source** — free, MIT, full tool.
  2. **Sponsor** — support development (GitHub Sponsors link).
  3. **Teams** — issues, docs, priority support via the repo.
- **Footer** — minimal monospace repo links, `border-t border-white/10`,
  thin carbon dividers.

## Files

~8 source files:

```
index.html
vite.config.ts
src/index.css        # tailwind + fonts
src/App.tsx
src/scene/Scene.tsx
src/hooks/useScrollProgress.ts
src/components/{Nav,Hero,Features,Pricing,Footer}.tsx
```

## Constraints

- Procedural geometry only (drei/three primitives); no `.gltf`/`.obj`.
- No image placeholders — canvas, icons, typography only.
- Fully responsive: hero collapses to one column, bento stacks, knot
  recenters on small screens.
- Zero server logic; static export only.

## Verification

- `npm run build` clean; `docs/` output committed on `gh-pages`.
- Serve `docs/` locally, screenshot-check hero/features/pricing in a
  browser (glass blur, lights, parallax, scroll transitions) before push.
- After push: site live at `ntxinh.github.io/SchemaDiff/`.

## Non-goals

- No CI workflow for the site (docs/ carry-in is the deploy).
- No multi-page routing, blog, or docs site.
- No real payment/pricing backend — tiers are links, not checkout.
