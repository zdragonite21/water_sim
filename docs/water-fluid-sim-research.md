# Toward a High-End Real-Time Water Simulator

This document is a research and architecture guide for building a real-time water
simulation/rendering project in Rust with `wgpu`/WebGPU first, while keeping a
desktop Vulkan path open for higher-end features later.

The target is ambitious: convincing local splashes, whitewater, spray, caustics,
refraction, reflection, shadows, god rays, and eventually large-scale oceans. The
main conclusion is that one solver will not do everything well. The strongest
architecture is a hybrid: spectral ocean for far field water, cheaper 2D wave
methods for mid-field interaction, local 3D particle/grid simulation near the
viewer, and a rendering system that sells the optical behavior of water.

## Executive Summary

If the goal is visual realism in real time, do not start by chasing full
physical correctness. Start by making the water render correctly, then add local
simulation where the viewer can inspect it.

Recommended first serious stack:

1. Far field ocean: FFT or Gerstner/spectrum waves.
2. Mid field interaction: wave particles, shallow-water wakes, and foam maps.
3. Near field simulation: GPU PBF/SPH/DFSPH-style local patch for splashes and
   pours, or FLIP/APIC later if you want a more VFX-style solver.
4. Whitewater: secondary particles for foam, bubbles, spray, mist, and droplets.
5. Rendering: Fresnel reflection, refraction, absorption, depth-based color,
   screen-space or planar reflections, shadowed transparent water, projected
   caustics, volumetric light shafts, and foam/spray shading.
6. Later desktop path: Vulkan and hardware ray tracing for expensive dynamic
   reflection/refraction or ray-traced caustics after the raster path already
   looks good.

The expensive part changes by scene:

- Small detailed splash: simulation, neighbor search, pressure solving,
  particle sorting, surface reconstruction, and whitewater emission can dominate.
- Ocean scene: rendering usually dominates, especially reflections, refraction,
  transparency, foam, caustics, volumetric lighting, and screen-space effects.
- Voxel sandbox: memory bandwidth dominates. Every cell update is simple, but
  there are many cells, and 3D grids grow brutally fast.

The strongest long-term idea from your prompt is the ocean LOD blend:

- FFT ocean everywhere.
- A local 3D simulated patch around the player/boat/camera.
- Wave particles and wake fields as the bridge.
- Foam/whitewater maps as the perceptual glue.
- Two-way influence only where it matters visually.

That is very close to the recent paper
[Real-Time Interactive Hybrid Ocean: Spectrum-Consistent Wave Particle-FFT Coupling](https://arxiv.org/abs/2511.02852),
which couples a global FFT ocean with local wave-particle patches under one
spectrum.

## The Mental Model

Water has two jobs in a game or interactive graphics demo:

1. The simulation decides where the water is and how it moves.
2. The renderer convinces your eyes that the thing is water.

Those are different problems.

Like you are 12: imagine a magician. The simulation is the hidden machinery
under the stage. The renderer is the trick the audience sees. If the machinery
is perfect but the trick is lit badly, nobody believes it. If the machinery is
simple but the lighting, reflections, foam, and spray are excellent, people
believe it immediately.

This matters because physically accurate water is extremely expensive. Real
water contains waves, bubbles, droplets, mist, light bending, light focusing,
turbulence, foam, suspended particles, and interaction across huge scale ranges.
A real-time project must choose which pieces are simulated and which pieces are
rendering approximations.

## What Makes Water Look Like Water

A convincing water renderer usually needs these properties:

- Fresnel reflection: water reflects more at grazing angles. Looking across a
  lake gives strong reflection; looking straight down lets you see through it.
- Refraction: light bends as it enters/exits water, so underwater objects appear
  shifted.
- Absorption: water color changes with depth because red wavelengths disappear
  faster, leaving blue/green.
- Scattering: suspended particles and tiny bubbles bounce light around, creating
  haze and softness.
- Surface normals: small ripples matter enormously because they control
  reflection/refraction direction.
- Foam and bubbles: whitewater tells the viewer where water is breaking,
  colliding, and mixing with air.
- Spray and mist: airborne particles communicate energy and scale.
- Caustics: bright dancing light patterns caused by waves focusing light.
- Shadows: water needs to receive and cast shadows in plausible ways.
- God rays: underwater volumetric shafts caused by forward-scattered light.

For real-time rendering, most of these are approximated. That is normal. The
question is not "is it exact?" The question is "does the approximation preserve
the visual cue the eye expects?"

## Simulation Families

### FFT and Spectral Oceans

FFT oceans model the ocean as a sum of many waves in frequency space. Instead of
simulating every droplet, you generate a height field from a wave spectrum and
animate it with physically motivated wave dispersion.

Foundational reference:
[Tessendorf, Simulating Ocean Water](https://people.computing.clemson.edu/~jtessen/reports/papers_files/coursenotes2004.pdf).

Practical GPU-era reference:
[GPU Gems Chapter 1, Effective Water Simulation from Physical Models](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models).

Why use it:

- It is the best tool for large oceans.
- It gives convincing broad swell cheaply.
- It maps naturally to GPU textures and compute.
- It can use real-ish spectra such as Phillips, Pierson-Moskowitz, or JONSWAP.

Why not use it for everything:

- It is mostly a height field. It cannot naturally represent overturning waves,
  caves, droplets, buckets of water, or full 3D splashes.
- It assumes a broad statistically uniform ocean unless extended.
- Local interaction is awkward. A boat wake or player splash has to be injected
  as extra waves, decals, particles, or a local simulation patch.

Best use:

- Far ocean.
- Large lakes.
- Main wave motion under boats.
- Visual continuity behind higher-detail local simulation.

### Gerstner Waves

Gerstner waves are analytic waves that move vertices sideways as well as up and
down. This creates sharper crests than simple sine waves.

GPU Gems Chapter 1 explains why Gerstner waves are useful for sharper crests and
controllable water surfaces:
[GPU Gems Chapter 1](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models).

Why use them:

- Very cheap.
- Art-directable.
- Easy to implement in vertex shaders or compute.
- Good for stylized or game-like oceans.

Tradeoff:

- They are not a full fluid solver.
- They can self-intersect if steepness is pushed too far.
- They are best as a surface animation layer, not a splash simulator.

Best use:

- First ocean prototype.
- Normal-map detail.
- Controlled waves in gameplay areas.

### SPH

SPH means Smoothed Particle Hydrodynamics. You represent water as particles.
Each particle samples nearby particles through a smoothing kernel.

Like you are 12: each particle asks, "who is near me?" Then it estimates local
density and pressure from its neighbors. If too many particles are packed
together, pressure pushes them apart.

Good references:

- [Koschier, Bender, Solenthaler, Teschner, Smoothed Particle Hydrodynamics Techniques](https://arxiv.org/abs/2009.06944)
- [Bender and Koschier, Divergence-Free Smoothed Particle Hydrodynamics](https://animation.rwth-aachen.de/media/papers/2015-SCA-DFSPH.pdf)
- Interactive demo/explanation: [DFSPH example](https://interactivecomputergraphics.github.io/physics-simulation/examples/dfsph.html)

Why use SPH:

- Natural free surfaces.
- Good for splashes, droplets, pouring, and arbitrary shapes.
- Conceptually easier than grid pressure solvers at first.
- Sparse: you only pay for where particles exist.

Why SPH is hard:

- Neighbor search is expensive.
- Incompressibility is expensive.
- Boundary handling is tricky.
- Surface reconstruction is a separate rendering problem.
- Naive SPH can look blobby or compressible.

Real-time variants:

- WCSPH: weakly compressible SPH. Simpler, but water may look springy.
- PCISPH/IISPH: iterative pressure correction for better incompressibility.
- DFSPH: enforces both low density error and low velocity divergence. Strong
  choice for serious particle water.

Best use:

- Local 3D fluid around the player.
- Splash tank demos.
- Droplets and interactive water volumes.
- Whitewater source simulation.

### Position Based Fluids

PBF formulates incompressibility as position constraints in the Position Based
Dynamics style. Instead of applying forces and hoping the fluid settles, it
iteratively moves particle positions until density is closer to the target.

Reference:
[Macklin and Muller, Position Based Fluids](https://mmacklin.com/pbf_sig_preprint.pdf).

Project page:
[Miles Macklin, PBF project](https://blog.mmacklin.com/project/pbf/).

Why PBF is popular in games:

- Stable.
- Allows large-ish time steps.
- Easier to integrate with other position-based constraints.
- Good visual results for real-time particles.

Tradeoffs:

- Not as physically pure as more rigorous solvers.
- Iteration count controls stiffness/compressibility.
- It can lose energy, so vorticity confinement and viscosity fixes are commonly
  added.

Best use:

- First GPU particle water solver.
- Interactive splashes.
- Game-like local fluid effects.

### FLIP, PIC, and APIC

PIC and FLIP are particle-grid methods. Particles carry material, but a grid
does the pressure solve.

Like you are 12: particles are the water beads, but every frame they report
their motion to a temporary invisible grid. The grid solves the "do not squash
water" problem, then gives corrected motion back to the particles.

Important references:

- Bridson's book/course material is still the classic practical path:
  [Fluid Simulation for Computer Graphics](https://www.cs.ubc.ca/~rbridson/fluidsimulation/)
- APIC reference:
  [Jiang, Schroeder, Teran, The Affine Particle-In-Cell Method](https://arxiv.org/abs/1603.06188)

Why use FLIP/APIC:

- Common in offline/VFX liquid simulation.
- Handles incompressibility well.
- Good for big splashes and complex free surfaces.
- APIC reduces some noisy/diffusive behavior in particle-grid transfers.

Why it is harder for your first WebGPU implementation:

- You need particle-to-grid and grid-to-particle transfers.
- You need a pressure projection solve.
- You need sparse/narrow-band grids for performance.
- GPU implementation requires careful prefix sums, atomics, sorting, and memory
  layout.

Best use:

- More advanced desktop path.
- VFX-style local patch.
- High-quality splashes after you already have GPU infrastructure.

### MPM

MPM, the Material Point Method, is another particle-grid method. It is famous in
graphics for snow, mud, sand, elastoplastic solids, and multiphase material.

Why consider it:

- Excellent for materials that are not just water.
- Natural bridge between fluid, sand, snow, mud, and deformable material.
- Interesting if you want "falling sand plus fluid plus solids" in 3D.

Why not start there for pure water:

- It is more general than you need.
- Clean water alone is usually easier with SPH/PBF/DFSPH or FLIP/APIC.

Best use:

- Multimaterial sandbox.
- Mud/sand/snow/water hybrid experiments.

### Voxel, Cellular, and Falling-Sand-Style Water

Noita-like falling sand works so well in 2D because a 2D grid is cheap and
material rules can be local. In 3D, the same idea becomes much heavier.

Like you are 12: a 2D grid is like a sheet of graph paper. A 3D grid is like a
huge stack of graph paper. Doubling resolution in 2D gives about 4x more cells.
Doubling resolution in 3D gives about 8x more cells.

For example:

- `128^3` = about 2.1 million cells.
- `256^3` = about 16.8 million cells.
- `512^3` = about 134 million cells.

Every frame, you may need to touch many of those cells. That is why memory
bandwidth becomes the wall.

Modern directions related to voxel water:

- Sparse grids, where empty space is not stored densely.
- Narrow-band grids, where only cells near the water surface get high detail.
- Lattice Boltzmann methods, which are local and GPU-friendly.
- Hybrid voxel + particle systems, where the grid handles bulk material and
  particles handle spray/dust/airborne detail.
- OpenVDB/NanoVDB-style sparse volume structures for offline and GPU rendering.

References:

- [OpenVDB](https://www.openvdb.org/) for sparse volumetric data in film/VFX.
- [NanoVDB](https://developer.nvidia.com/nanovdb) for GPU-friendly sparse VDB.
- [Interactive 3D fluid simulation using Lattice Boltzmann Method](https://arxiv.org/abs/1912.04356).
- [Highly Efficient Lattice-Boltzmann Multiphase Simulations on CPUs and GPUs](https://arxiv.org/abs/2012.06144).

Practical advice:

- If you want Noita-in-3D material interaction, investigate sparse voxel chunks
  plus particles.
- If you want photoreal local splashes, start with particles or FLIP/APIC
  instead.
- If you want a web demo, keep the voxel domain small and chunked.

## Surface Reconstruction and Rendering the Fluid Shape

The simulation gives you particles, voxels, or a height field. The renderer
needs something to shade. This is one of the biggest design choices.

### Height Field

Used by FFT/Gerstner oceans.

Pros:

- Very fast.
- Easy normals.
- Easy LOD.
- Easy reflection/refraction.

Cons:

- Cannot represent overturns, droplets, tunnels, or full 3D splashes.

Best for:

- Oceans, lakes, rivers viewed mostly from above.

### Screen-Space Fluid Rendering

Particles are rendered into screen-space buffers: depth, thickness, maybe
velocity. Then the surface is smoothed and shaded.

PBF used GPU ellipsoid splatting plus screen-space filtering, referencing the
screen-space method by van der Laan et al. and anisotropic kernels by Yu and
Turk:
[Position Based Fluids](https://mmacklin.com/pbf_sig_preprint.pdf).

Pros:

- Great for real-time particle fluids.
- No full mesh rebuild.
- Good first renderer for SPH/PBF.
- Works nicely with transparency and thickness.

Cons:

- View-dependent.
- Harder to cast exact shadows/reflections from the reconstructed surface.
- Can fail at silhouettes or thin sheets.

Best for:

- First high-quality local fluid renderer.
- WebGPU experiments.

### Marching Cubes

Marching cubes extracts triangles from a 3D scalar field.

Original paper:
[Lorensen and Cline, Marching Cubes](https://dl.acm.org/doi/10.1145/37402.37422).

Like you are 12: imagine each voxel cube asking, "does the water surface pass
through me?" If yes, a lookup table tells it which little triangles to create.

Pros:

- Produces ordinary triangles.
- Easy to shade with normal rendering pipelines.
- Can feed into BVHs, shadow maps, reflection probes, and ray tracing.
- Good for offline or high-end local patches.

Cons:

- You must build a scalar field first.
- Rebuilding a mesh every frame can be expensive.
- Topology can flicker if not smoothed/filtered carefully.
- Dynamic BVH rebuild can be expensive.

Best for:

- High-quality desktop local fluid patch.
- Native Vulkan path.
- When you need actual geometry for ray tracing or shadows.

### Raymarching SDFs or Density Fields

Instead of converting to triangles, raymarch through a signed distance field
or density field.

Pros:

- Smooth implicit surfaces.
- No mesh generation.
- Good for volumes, bubbles, foam clouds, mist, and stylized water.
- Can be natural in compute shaders.

Cons:

- Empty-space skipping is essential.
- Large volumes get expensive.
- Refraction/reflection from raymarched surfaces needs careful normal
  estimation.

Best for:

- SDF particle fields.
- Mist/foam volumes.
- Small contained simulations.

### Gaussian Splatting-Style Rendering

Recent graphics work is exploring particle/gaussian representations for dynamic
scenes and fluids.

Reference:
[Gaussian Splashing: Unified Particles for Versatile Motion Synthesis and Rendering](https://arxiv.org/abs/2401.15318).

Project/demo page:
[Gaussian Splashing](https://gaussiansplashing.github.io/).

This is not the obvious first path for a WebGPU water sim, but it is worth
watching. It points toward a future where particle-like render representations
can handle complex dynamic scenes without conventional meshing.

## Whitewater, Foam, Spray, Bubbles, and Mist

Whitewater is visually essential. It communicates energy, scale, turbulence, and
air-water mixing.

A practical real-time system should not fully simulate air-water two-phase flow.
Instead, use secondary particles and maps.

### Categories

Foam:

- Lives on or near the water surface.
- Moves with the surface velocity.
- Fades slowly.
- Collects in troughs, shorelines, and behind obstacles.

Bubbles:

- Live underwater.
- Rise due to buoyancy.
- Scatter light strongly.
- Can feed foam when they reach the surface.

Spray:

- Airborne water droplets.
- Ballistic motion plus drag.
- Fades or falls back into water.
- Strongly tied to impacts and breaking crests.

Mist:

- Tiny droplets.
- Rendered as soft particles or volumes.
- Important for waterfalls, big waves, and stormy ocean scale.

### Emission Signals

Emit whitewater when:

- Velocity magnitude is high.
- Relative collision velocity is high.
- Vorticity is high.
- Surface curvature is high.
- A wave crest exceeds a steepness threshold.
- Water hits solid geometry.
- Water particles separate from dense neighbors.
- Air is entrained by plunging/breaking motion.

Recent physical references for why bubbles and spray matter:

- [Large Eddy Simulations of bubbly flows and breaking waves with SPH](https://arxiv.org/abs/2206.01641).
- [The turbulent bubble break-up cascade. Part 2. Numerical simulations of breaking waves](https://arxiv.org/abs/2009.04804).

Real-time simplification:

- Do not simulate every real bubble.
- Use the main fluid to emit representative particles.
- Render them with good lighting, soft depth, and temporal stability.
- Let foam be a texture/particle layer driven by physical-ish signals.

### Rendering Whitewater

Foam rendering options:

- Surface foam texture on ocean height field.
- Foam particles projected onto the surface.
- Screen-space foam mask from curvature/depth/velocity.
- Decal or texture atlas advected by flow.

Spray rendering options:

- Instanced billboard particles.
- Lit soft particles with depth fade.
- Tiny point sprites for distant droplets.
- Volumetric mist for dense spray clouds.

Bubbles:

- Underwater particles.
- Screen-space spheres.
- Volume density field.
- Bright rim/high scattering shading.

The biggest trick: whitewater should be lit as a participating medium-ish
object, not as flat white decals. Foam catches light, self-shadows, and fades
with thickness.

## Caustics

Caustics are bright light patterns caused by reflection/refraction focusing
light.

Like you are 12: water waves act like moving magnifying glasses. Some spots get
extra sunlight because many bent rays land there.

Real-time caustics are usually fake, and that is fine.

### Projected Caustic Texture

Your idea of rendering caustics into a flat texture and projecting it is one of
the best practical approaches.

Reference:
[GPU Gems Chapter 2, Rendering Water Caustics](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-2-rendering-water-caustics).

Basic method:

1. Treat the sun/light as a source.
2. Use the water surface normal to refract light rays.
3. Estimate where refracted rays hit a receiver plane or scene depth.
4. Accumulate intensity into a caustic map.
5. Project/sample that caustic map during lighting.

Pros:

- Cheap.
- Looks excellent for shallow water.
- Works in WebGPU.
- Easy to stabilize temporally.

Cons:

- Usually assumes a receiver plane or simplified scene.
- Not physically exact for complex geometry.
- Needs careful filtering to avoid shimmer.

### Photon Mapping and Path Tracing

Offline caustics are often handled with photon mapping, bidirectional path
tracing, or related light transport methods. They are expensive because caustic
paths are rare and concentrated.

Classic reference:
[Jensen, Global Illumination using Photon Maps](https://graphics.stanford.edu/~henrik/papers/ewr7/egwr96.pdf).

Real-time advice:

- Use projected caustics first.
- Use ray tracing later for selected desktop effects.
- Do not make caustics your first hardware RT milestone.

## Reflection and Refraction

Water reflection/refraction is often more important than the sim.

### Reflection Methods

Planar reflection:

- Render the scene mirrored across the water plane.
- Excellent for flat-ish lakes/oceans.
- Expensive but stable.

Screen-space reflection:

- Trace reflection rays through the already rendered frame.
- Cheap-ish and dynamic.
- Fails for off-screen objects.

Reflection probes/cubemaps:

- Cheap and stable.
- Less accurate for nearby dynamic objects.

Ray tracing:

- Most accurate.
- Expensive.
- WebGPU core does not expose hardware ray tracing today.
- Vulkan/DX12 gives you the native path later.

Recommended path:

- Start with sky/environment reflection plus SSR.
- Add planar reflection for calm water or hero shots.
- Add native ray tracing only after the raster water is strong.

### Refraction Methods

Screen-space refraction:

- Use water normal and thickness/depth to offset background sampling.
- Cheap and practical.

Thickness-based absorption:

- Estimate how far light travels through water.
- More thickness means deeper color and less visibility.

Underwater view:

- Add fog/scattering.
- Add caustics.
- Add god rays.
- Distort view with surface normals.

## Shadows and God Rays

Shadows on water are hard because transparent materials both receive and transmit
light.

Practical real-time approach:

- Water surface receives shadows normally, but attenuated.
- Underwater objects receive caustics plus filtered shadows.
- Foam/spray particles receive shadow maps.
- Mist uses volumetric shadow approximation.

God rays:

- Best approximated as volumetric light shafts.
- In shallow water, modulate them by caustic intensity and surface normal.
- In deep water, use depth fog and anisotropic scattering.

Like you are 12: underwater god rays are sunlight beams made visible because the
water is not perfectly clear. Tiny particles and bubbles scatter some of the
light toward your eyes.

## Scale and LOD

This is the most important architecture section.

You cannot simulate a whole ocean as 3D particles. You also cannot fake a bucket
of water with only FFT waves. The solution is LOD by physical regime.

### Proposed Scale Stack

Far field:

- FFT/spectral ocean.
- Cascaded wave textures.
- Low-frequency displacement.
- High-frequency normal maps.
- Distant foam/whitecap textures.

Mid field:

- Wake maps.
- Wave particles.
- Shallow-water solver for local height-field interaction.
- Foam accumulation maps.

Near field:

- 3D PBF/SPH/DFSPH or FLIP/APIC patch.
- Particle spray.
- Bubbles and foam.
- Screen-space or mesh surface.

Very near / hero:

- Higher particle density.
- More whitewater.
- Better thickness/refraction.
- Optional local mesh/raymarch surface.

### Blending FFT Ocean with Local 3D Simulation

Your idea:

> Combine FFT ocean with a small region around the player simulated in 3D with
> particles, blend between 3D and FFT, and allow influence through forces.

This is the right direction.

Practical coupling:

1. The FFT ocean provides boundary height and velocity for the local patch.
2. Local particles are spawned around the player/object where interaction occurs.
3. At the patch boundary, particles are damped or converted back into wave/foam
   data.
4. The local patch writes disturbances into wake maps or wave particles.
5. Foam and spray hide the blend region.

Recent reference:
[Real-Time Interactive Hybrid Ocean: Spectrum-Consistent Wave Particle-FFT Coupling](https://arxiv.org/abs/2511.02852).

Important reasoning:

- Full two-way physics coupling is expensive.
- Perceptual coupling is often enough.
- The viewer cares that a boat leaves a wake, splashes happen near the hull, and
  the far ocean remains coherent.

### LOD for Whitewater

Whitewater also needs LOD:

- Far: whitecap texture/noise from wave steepness.
- Mid: foam maps and wake streaks.
- Near: foam particles and bubble particles.
- Hero: spray, mist, droplets, and shadowed particles.

Do not render distant spray as thousands of particles. Convert it to mist volume,
screen-space density, or texture layers.

## CPU vs GPU Simulation

### CPU Strengths

CPU simulation is good when:

- Particle count is modest.
- Logic is branchy.
- Determinism matters.
- Gameplay coupling is strong.
- Debuggability matters.
- You want the GPU free for heavy rendering.

This matches the style of single-core/small-CPU demos you mentioned. If the
simulation count is modest and clever, leaving the GPU for rendering can be a
valid strategy.

CPU drawbacks:

- Millions of particles are hard.
- Dense 3D grids are hard.
- CPU-to-GPU transfer every frame can become a bottleneck.
- Single-core designs hit a ceiling quickly.

### GPU Strengths

GPU simulation is good when:

- Work is massively parallel.
- Many particles/cells do similar operations.
- Data can stay on the GPU.
- You need sorting, prefix sums, compaction, meshing, raymarching, or particle
  rendering.

GPU drawbacks:

- Harder to debug.
- Requires careful memory layout.
- Many algorithms need several passes.
- Atomics and synchronization can be tricky.
- WebGPU has portability and validation constraints.

### The Practical Split

Good CPU responsibilities:

- Scene/gameplay state.
- Emitters.
- Collision proxy generation.
- UI/debug controls.
- Parameter tuning.
- Small wave particle systems.

Good GPU responsibilities:

- Particle integration.
- Spatial hashing.
- Neighbor search.
- Pressure iterations.
- Grid solves.
- Foam/spray particle updates.
- Caustic map generation.
- Screen-space fluid rendering.
- Marching cubes or raymarching.

The best rule:

Keep high-volume data on the GPU once it is there. Avoid CPU-GPU ping-pong.

## Your Proposed Architecture: CPU Sim + GPU Marching Cubes + BVH + Ray Tracing

This is a valid high-end architecture, but I would not build it first.

Pipeline:

1. CPU sim updates particles or voxels.
2. Upload fluid data to GPU.
3. GPU converts particles/voxels into scalar field.
4. GPU marching cubes builds triangles.
5. GPU builds or refits BVH.
6. Ray tracing shades reflection/refraction/shadows.

Advantages:

- Leaves CPU simulation simple/debuggable.
- Produces actual geometry.
- BVH/ray tracing can give high realism.
- Good match for desktop Vulkan/DX12.

Main risks:

- CPU-to-GPU upload cost.
- Marching cubes every frame can be expensive.
- Dynamic BVH build can be expensive.
- Ray tracing transparent water is not automatically easy; refraction and
  caustics still need careful sampling/denoising.
- WebGPU core is not the right hardware-RT API today.

Better staged version:

1. CPU or GPU PBF particles.
2. Screen-space fluid rendering first.
3. Add projected caustics and foam.
4. Add optional GPU scalar field and marching cubes for small local hero areas.
5. Add Vulkan RT path only for desktop.

## WebGPU/wgpu vs Vulkan

`wgpu` is a good choice for this project because it gives you Rust ergonomics,
native backends, and a web path. The official `wgpu` repository describes it as
a cross-platform Rust graphics API running on Vulkan, Metal, D3D12, OpenGL, and
WebGPU/WebGL on wasm:
[gfx-rs/wgpu](https://github.com/gfx-rs/wgpu).

WebGPU specification:
[W3C WebGPU](https://www.w3.org/TR/webgpu/).

### Why Start with WebGPU/wgpu

- You can share demos on the web.
- Compute shaders are available.
- Rust integration is strong.
- The API teaches modern explicit GPU thinking.
- Most raster/compute water techniques fit.

Good WebGPU targets:

- FFT/Gerstner ocean.
- Projected caustics.
- Screen-space fluid rendering.
- PBF/SPH particle sim.
- Foam/spray particles.
- Simple raymarching.
- GPU sorting/prefix sums.

### Where Vulkan Wins

Use Vulkan later if you need:

- Hardware ray tracing.
- More control over subgroups/wave ops.
- Advanced synchronization and memory control.
- Vendor-specific performance tuning.
- More mature native profiling/debugging for extreme GPU workloads.

The practical recommendation:

- Build the engine in `wgpu`.
- Keep abstractions clean enough that a native Vulkan renderer can replace or
  augment the backend later.
- Do not start with Vulkan unless your first milestone explicitly requires
  hardware ray tracing.

## Recommended Build Roadmap

### Phase 1: Beautiful Static/Height-Field Water

Goal: make a water plane/ocean look good before real fluid sim.

Implement:

- Camera + sky.
- FFT or Gerstner waves.
- Dynamic normal maps.
- Fresnel reflection.
- Screen-space or environment reflection.
- Screen-space refraction.
- Depth-based absorption.
- Foam from wave steepness.
- Projected caustics.

Why:

- This creates the visual foundation.
- Later simulation can plug into the renderer.
- It produces shareable results quickly.

### Phase 2: Wave Interaction

Implement:

- Wake map.
- Disturbance stamps.
- Wave particles or shallow-water ripples.
- Object/player interaction.
- Foam accumulation.

Why:

- Gives interactivity without full 3D fluid cost.
- Bridges ocean and local sim.

### Phase 3: GPU Particle Infrastructure

Implement:

- Particle buffers.
- Spatial hash grid.
- Sorting.
- Prefix sums/scan.
- Compaction.
- Indirect draw or instancing.
- Debug visualizers.

Why:

- This infrastructure is required for SPH/PBF, spray, foam, and many other
  systems.

### Phase 4: Local PBF/SPH Patch

Implement:

- PBF or DFSPH-inspired local solver.
- Boundary collisions.
- Screen-space fluid rendering.
- Thickness buffer.
- Basic whitewater emission.

Why:

- This gives actual 3D local water.
- PBF is easier than FLIP/APIC as a first GPU solver.

### Phase 5: Whitewater System

Implement:

- Foam particles.
- Spray particles.
- Bubbles.
- Mist.
- Different lifetimes/materials.
- Emission from curvature, vorticity, impacts, and wave steepness.

Why:

- Whitewater sells the motion and hides blending artifacts.

### Phase 6: Hybrid Ocean LOD

Implement:

- FFT ocean around the world.
- Local 3D patch around the viewer/object.
- Boundary coupling from ocean to local patch.
- Wake/foam write-back from local patch to ocean maps.
- LOD transitions.

Why:

- This is the architecture that supports both small fidelity and vast scale.

### Phase 7: Native High-End Path

Implement:

- Optional Vulkan backend.
- GPU marching cubes for local patches.
- Dynamic BVH for selected fluid geometry.
- Ray-traced reflection/refraction experiments.
- Temporal denoising.

Why:

- This is where desktop realism opens up.
- It should come after the core water already looks good.

## Architecture Combinations

### Web-Shareable Beautiful Ocean

Use:

- FFT/Gerstner waves.
- Cascaded normal maps.
- SSR/environment reflection.
- Screen-space refraction.
- Projected caustics.
- Foam maps.
- Spray particles near impacts.

Pros:

- Best early demo.
- Runs on WebGPU.
- Visually strong.

Cons:

- Not a true 3D fluid sim.
- Limited breaking waves.

### Small High-Fidelity Splash Tank

Use:

- GPU PBF/SPH/DFSPH local sim.
- Screen-space fluid rendering.
- Secondary foam/spray/bubble particles.
- Projected caustics.
- Optional SDF/raymarch surface.

Pros:

- Great learning project.
- Strong local interaction.
- Good WebGPU fit.

Cons:

- Does not solve vast oceans.
- Needs careful particle infrastructure.

### Desktop Cinematic Local Water

Use:

- FLIP/APIC or DFSPH.
- Narrow-band scalar field.
- GPU marching cubes.
- BVH/ray tracing.
- Volumetric mist.
- High-quality temporal filtering.

Pros:

- Highest realism.
- Actual geometry.
- Good for native Vulkan.

Cons:

- Big implementation.
- Harder to make web-shareable.

### 3D Voxel Sandbox Water

Use:

- Sparse voxel chunks or LBM/free-surface grid.
- Particle spray.
- Raymarched or meshed surface.
- Material interactions.

Pros:

- Best for Noita-like 3D material interaction.
- Good for gameplay/destruction.

Cons:

- Memory bandwidth is brutal.
- Photoreal water is harder than with particles/FLIP.

### CPU Sim + GPU Heavy Rendering

Use:

- CPU local solver with modest particle count.
- GPU screen-space or mesh renderer.
- GPU caustics/foam/spray.

Pros:

- Easier debugging.
- Frees GPU design for rendering.
- Can work well if particle count is modest.

Cons:

- CPU ceiling.
- Upload cost.
- Hard to scale to millions of particles.

## Research and Demo Reading List

Core ocean/waves:

- [Tessendorf, Simulating Ocean Water](https://people.computing.clemson.edu/~jtessen/reports/papers_files/coursenotes2004.pdf)
- [GPU Gems Chapter 1, Effective Water Simulation from Physical Models](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models)
- [Real-Time Interactive Hybrid Ocean: Spectrum-Consistent Wave Particle-FFT Coupling](https://arxiv.org/abs/2511.02852)
- [Water Surface Wavelets, Macklin et al. listing](https://blog.mmacklin.com/publications/)

Particle fluids:

- [Macklin and Muller, Position Based Fluids](https://mmacklin.com/pbf_sig_preprint.pdf)
- [PBF project page](https://blog.mmacklin.com/project/pbf/)
- [Koschier et al., SPH Techniques for Fluids and Solids](https://arxiv.org/abs/2009.06944)
- [Bender and Koschier, DFSPH](https://animation.rwth-aachen.de/media/papers/2015-SCA-DFSPH.pdf)
- [Interactive DFSPH example](https://interactivecomputergraphics.github.io/physics-simulation/examples/dfsph.html)

Particle-grid / VFX-style simulation:

- [Bridson, Fluid Simulation for Computer Graphics](https://www.cs.ubc.ca/~rbridson/fluidsimulation/)
- [Jiang, Schroeder, Teran, APIC](https://arxiv.org/abs/1603.06188)

Voxel/sparse/grid directions:

- [OpenVDB](https://www.openvdb.org/)
- [NanoVDB](https://developer.nvidia.com/nanovdb)
- [Interactive 3D fluid simulation using LBM](https://arxiv.org/abs/1912.04356)
- [Highly Efficient Lattice-Boltzmann Multiphase Simulations](https://arxiv.org/abs/2012.06144)

Surface reconstruction/rendering:

- [Lorensen and Cline, Marching Cubes](https://dl.acm.org/doi/10.1145/37402.37422)
- [Gaussian Splashing paper](https://arxiv.org/abs/2401.15318)
- [Gaussian Splashing project page](https://gaussiansplashing.github.io/)

Caustics/light transport:

- [GPU Gems Chapter 2, Rendering Water Caustics](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-2-rendering-water-caustics)
- [Jensen, Global Illumination using Photon Maps](https://graphics.stanford.edu/~henrik/papers/ewr7/egwr96.pdf)

Whitewater/bubbles/breaking waves:

- [Large Eddy Simulations of bubbly flows and breaking waves with SPH](https://arxiv.org/abs/2206.01641)
- [The turbulent bubble break-up cascade. Part 2](https://arxiv.org/abs/2009.04804)
- [Droplet Simulations in Computer Graphics: Theories, Methods and Applications](https://arxiv.org/abs/2411.15880)

WebGPU/wgpu:

- [W3C WebGPU specification](https://www.w3.org/TR/webgpu/)
- [gfx-rs/wgpu](https://github.com/gfx-rs/wgpu)

## Final Recommendation

For this project, the highest-value path is not "pick the perfect solver." It is
to build a layered water system where each method is used at the scale where it
is strongest.

Start with `wgpu` and WebGPU:

- You will learn the right compute/render architecture.
- You can share demos on the web.
- You can implement most important real-time techniques.

Do not start with hardware ray tracing:

- It will not solve the hardest water problems by itself.
- Dynamic fluid geometry and caustics are still expensive.
- WebGPU does not give you the native RT path anyway.

Build the renderer first, then local simulation:

- Fresnel, reflection, refraction, absorption, foam, caustics, and spray will
  make even simple waves look alive.
- Once the renderer is strong, every simulation improvement becomes visible.

The endgame architecture should be:

- FFT/spectral ocean for vast scale.
- Wave particles/wake fields for mid-scale interaction.
- Local 3D PBF/SPH/DFSPH or FLIP/APIC patch for hero interaction.
- Secondary whitewater particles everywhere water breaks.
- Projected caustics and volumetric light for optical richness.
- Optional Vulkan hardware ray tracing for desktop hero mode.

That gives you a realistic path toward both goals: a shareable WebGPU water demo
and a future desktop renderer that can push much harder.
