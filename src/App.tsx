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
