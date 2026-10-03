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
