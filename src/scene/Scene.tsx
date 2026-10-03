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
