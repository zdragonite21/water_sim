# Recent Water and Fluid Simulation Breakthroughs, 2020-2026

This document surveys recent work from 2020 through mid-2026 that matters for a
high-end water simulation project. It includes true water/ocean/liquid papers,
nearby fluid simulation papers, rendering papers, and adjacent techniques that
could become useful in a Rust + `wgpu`/WebGPU simulator or a later Vulkan
desktop renderer.

The scope is intentionally broad. A paper does not need to be "game water" to be
useful. For example, a paper about particle-laden turbulence can still teach us
how to fake spray and mist; a quadtree flood model can teach us how to allocate
resolution only where water actually needs it; a neural flow-map method can
suggest how to preserve vortices without burning energy away.

## Big Picture

The recent breakthroughs are not one single magic solver. They cluster into a
few ideas:

- Flow maps: remember where fluid came from and where it is going, so advection
  loses less detail.
- Adaptive grids: use fine cells only where detail exists. Quadtrees, octrees,
  sparse blocks, and narrow bands all belong to this family.
- Wavelets/subgrid models: add plausible missing small-scale motion without
  simulating every tiny turbulent eddy.
- Hybrid ocean methods: combine FFT oceans for scale with local wave particles
  or patches for interaction.
- Neural/differentiable simulation: train or optimize simulation components,
  often for speed, control, inverse design, or learned turbulence.
- Better interface tracking: preserve the moving water-air surface with less
  volume loss and less smoothing.
- Better water perception/rendering: realism is not just fluid motion; it is
  reflection, refraction, caustics, bubbles, foam, scattering, and sensor/visual
  effects.

Like you are 12: the newest work is mostly about cheating smarter. Instead of
simulating every molecule of water, researchers ask: "What detail does the eye
need? Where should we spend resolution? Can we remember motion better? Can we
learn the tiny stuff instead of calculating all of it?"

## Technique Map

### Flow Maps

A flow map is a memory of fluid motion.

Imagine putting invisible name tags on water. After one second, each tag moved
somewhere else. A flow map says, "the water currently here came from over
there." That is useful because many fluid solvers blur or smear details when
they move smoke/water/vortices around. If you can track the motion map more
accurately, you keep curls and features longer.

Most relevant recent sequence:

- 2023: [Fluid Simulation on Neural Flow Maps](https://arxiv.org/abs/2312.14635)
- 2024: [Eulerian-Lagrangian Fluid Simulation on Particle Flow Maps](https://arxiv.org/abs/2405.09672)
- 2025: [Fluid Simulation on Vortex Particle Flow Maps](https://arxiv.org/abs/2505.21946)
- 2025: [An Adjoint Method for Differentiable Fluid Simulation on Flow Maps](https://arxiv.org/abs/2511.01259)
- 2026: [A Level Set Method on Particle Flow Maps](https://arxiv.org/abs/2601.09939)

Why this matters for water:

- Vortices are important for splashes, foam, and spray.
- Less numerical smearing means more lively water.
- Flow maps may become useful for local high-quality simulation patches.
- Level-set flow maps are directly relevant to tracking the water-air surface.

### Quadtrees, Octrees, Sparse Blocks, and Adaptive Grids

A quadtree is a 2D grid that can split cells into four smaller cells. An octree
is the 3D version, splitting cells into eight. Sparse blocks are a related idea:
store high-resolution chunks only where needed.

Like you are 12: do not draw a whole city map at house-level detail if most of
the map is empty ocean. Draw tiny streets only where the player is standing, and
use big blocks everywhere else.

Relevant recent work:

- 2020: [An adaptive central-upwind scheme on quadtree grids for variable density shallow water equations](https://arxiv.org/abs/2008.02111)
- 2024: [GeoFlood: Computational model for overland flooding](https://arxiv.org/abs/2403.15435)
- 2026: [Adaptive GPU Kinetic Solver for Fluid-Granular Flows](https://arxiv.org/abs/2603.14982)

Why this matters for water:

- Large worlds need adaptive resolution.
- Flooding/shoreline water is often shallow-water physics, not full 3D fluid.
- A WebGPU simulator can use tile/chunk ideas even if it does not implement full
  academic adaptive mesh refinement.

### Wavelets and Subgrid Detail

Wavelets are small localized waves. They are good for representing detail that
exists in one place and at one scale.

Like you are 12: Fourier/FFT waves are like saying "this whole song contains a
low note and a high note." Wavelets are like saying "this little splash sound
happens right here, for a short time." That locality is why wavelets are useful
for turbulence and detail.

Recent useful paper:

- 2023: [Wavelet based modeling of subgrid-scales in LES of particle-laden turbulent flows](https://arxiv.org/abs/2305.09521)

Older but important context:

- [Wavelet Turbulence for Fluid Simulation](https://www.cs.cornell.edu/~tedkim/wturb/) is older than this survey window, but it is the classic graphics idea:
  simulate low resolution, then synthesize believable high-frequency turbulence.

Why this matters for water:

- Spray, foam, mist, and turbulent small eddies are too expensive to fully
  simulate.
- Wavelet/subgrid ideas can drive procedural high-frequency velocity fields.
- For a game-like renderer, wavelets can add plausible turbulent detail to
  whitewater and mist even if the base simulation is coarse.

### Neural and Differentiable Fluids

Neural fluid work tries to replace or augment parts of the solver with learned
models. Differentiable simulation lets you ask, "how should I change the input
so the water does what I want?"

Like you are 12: a normal sim is a toy train going forward. A differentiable sim
also tells you which track pieces caused the train to end up where it did, so you
can adjust the track automatically.

Recent useful papers:

- 2020/2021: [Learning Incompressible Fluid Dynamics from Scratch](https://arxiv.org/abs/2006.08762)
- 2022: [Learned Turbulence Modelling with Differentiable Fluid Solvers](https://arxiv.org/abs/2202.06988)
- 2022: [Fluid Simulation System Based on Graph Neural Network](https://arxiv.org/abs/2202.12619)
- 2023: [FluidLab](https://arxiv.org/abs/2303.02346)
- 2024: [Symmetric Basis Convolutions for Learning Lagrangian Fluid Mechanics](https://arxiv.org/abs/2403.16680)
- 2024: [A Pioneering Neural Network Method for Efficient and Robust Fluid Simulation](https://arxiv.org/abs/2412.10748)
- 2025: [An Adjoint Method for Differentiable Fluid Simulation on Flow Maps](https://arxiv.org/abs/2511.01259)

Why this matters for water:

- Learned models are not yet the safest core for a fully general water sandbox.
- They are promising for upres, denoising, foam/detail synthesis, solver
  acceleration, and control.
- Differentiable methods could tune wave parameters, caustic parameters, or
  emitter behavior automatically.

### Hybrid Oceans

The modern ocean direction is not "FFT or particles." It is "FFT for scale,
local methods for interaction."

Like you are 12: use a painted backdrop for the far ocean, but build a real
splash machine next to the camera.

Recent useful papers:

- 2025: [Arc Blanc: a real time ocean simulation framework](https://arxiv.org/abs/2503.03326)
- 2025: [Real-Time Interactive Hybrid Ocean: Spectrum-Consistent Wave Particle-FFT Coupling](https://arxiv.org/abs/2511.02852)

Why this matters for water:

- This lines up almost exactly with the desired LOD idea: far-field FFT plus
  local interaction patches.
- It is the strongest path for combining vast oceans with local realism.

## Year-by-Year Survey

## 2026

### A Level Set Method on Particle Flow Maps

Paper:
[A Level Set Method on Particle Flow Maps](https://arxiv.org/abs/2601.09939)

What it is:

This paper applies particle flow maps to level sets. A level set is a way to
store a surface as a distance field: negative inside, positive outside, zero at
the surface. In water simulation, that zero surface can represent the boundary
between water and air.

Like you are 12:

Imagine fogging up a glass box and drawing a glowing line where water meets air.
A level set is the computer's way of knowing where that glowing line is. The hard
part is that the line gets stretched, folded, and twisted. This paper tries to
move that line without losing tiny details.

Why it is interesting:

- It targets high-fidelity interface tracking.
- It stores level-set values, gradients, and Hessians on particles near the
  interface while still using a grid elsewhere.
- It reports strong volume preservation and shape fidelity.

Why it matters for water:

- Water rendering depends heavily on the surface.
- A bad interface tracker loses thin sheets, droplets, and splashes.
- This is more relevant to a future advanced desktop solver than a first WebGPU
  prototype, but the idea is important: spend accuracy near the interface.

Practical takeaway:

- For your project, treat "surface tracking" as its own major research problem.
- Even if you start with screen-space fluid rendering, later high-end water will
  need a better surface representation.

### Adaptive GPU Kinetic Solver for Fluid-Granular Flows

Paper:
[Adaptive GPU Kinetic Solver for Fluid-Granular Flows](https://arxiv.org/abs/2603.14982)

What it is:

This paper couples LBM for fluids with MPM for granular materials such as sand
and snow. It uses adaptive block-based multi-level grids on the GPU.

Like you are 12:

Water and sand are annoying together. Sand is like millions of tiny rocks. Water
is a flowing soup. This paper gives each material the kind of math it likes, then
connects them so they push on each other.

Why it is interesting:

- It targets large-scale fluid-granular interaction.
- It uses adaptive multi-level GPU blocks.
- It focuses on two-way coupling between fluid and granular materials.

Why it matters for water:

- For a 3D falling-sand/voxel-water direction, sand-water coupling is a core
  problem.
- It suggests that the future of large material interaction is hybrid, not one
  universal solver.
- The adaptive GPU block idea is relevant for sparse voxel water.

Practical takeaway:

- If you build a Noita-like 3D material sandbox, plan for hybrid solvers:
  voxel/LBM-ish water, particle/MPM-ish solids, and narrow high-resolution
  regions.

## 2025

### Real-Time Interactive Hybrid Ocean: Spectrum-Consistent Wave Particle-FFT Coupling

Paper:
[Real-Time Interactive Hybrid Ocean: Spectrum-Consistent Wave Particle-FFT Coupling](https://arxiv.org/abs/2511.02852)

What it is:

This is one of the most directly relevant papers for the ocean LOD idea. It
couples a global FFT ocean with local wave-particle patches around interactive
objects.

Like you are 12:

The far ocean is a big repeating orchestra. Local splashes are little musicians
near the player. The trick is making sure the little musicians play in the same
key as the big orchestra so the blend does not sound fake.

Why it is interesting:

- FFT gives large-scale ocean efficiently.
- Wave particles give local wakes and ripples.
- The paper focuses on spectrum consistency, meaning the local waves and global
  waves agree statistically.
- It uses frequency buckets for GPU-parallel synthesis.

Why it matters for water:

- This is close to the architecture you described.
- It directly addresses the "large ocean plus local interaction" problem.
- It suggests a realistic LOD path for WebGPU.

Practical takeaway:

- Build your ocean as a spectrum-driven system from the start.
- Let local interaction patches inherit the same spectrum/wind parameters.
- Hide patch boundaries with foam, spray, and matched wave statistics.

### Arc Blanc: a Real-Time Ocean Simulation Framework

Paper:
[Arc Blanc: a real time ocean simulation framework](https://arxiv.org/abs/2503.03326)

What it is:

Arc Blanc is a broad real-time ocean framework. It tries to explain how the
pieces of an ocean simulator connect: free surface, real-time velocity at depth,
and solid-fluid coupling.

Like you are 12:

Many papers give you one cool water part. This one is more like a machine manual
showing how the gears connect.

Why it is interesting:

- It is framework-oriented, not just a single trick.
- It discusses real-time ocean fluid velocity at any depth.
- It improves solid-to-fluid coupling input.

Why it matters for water:

- Object-water coupling is a hard part of interactive oceans.
- Depth velocity matters for floating objects, underwater particles, and
  coupling local effects to the ocean.

Practical takeaway:

- Do not treat the ocean as only a visible height field. Store/query velocity,
  slope, displacement, and foam-related quantities as first-class data.

### Fluid Simulation on Vortex Particle Flow Maps

Paper:
[Fluid Simulation on Vortex Particle Flow Maps](https://arxiv.org/abs/2505.21946)

What it is:

This extends the flow-map idea by focusing on vorticity. Vorticity is local
spinning motion in fluid.

Like you are 12:

Vorticity is the water's tiny spinning tornadoes. If your sim kills those tiny
tornadoes too quickly, splashes and turbulence look dead.

Why it is interesting:

- It evolves vorticity on particles.
- It reconstructs velocity on a background grid.
- It reports longer flow-map lengths and better vorticity preservation.
- It handles dynamic solid boundaries.

Why it matters for water:

- Whitewater and splash detail depend on turbulent rotational motion.
- Preserving vorticity can make water feel alive.
- This is more advanced than a first solver, but conceptually valuable.

Practical takeaway:

- Even in a simpler solver, track vorticity-like signals.
- Use vorticity as a whitewater emitter input.
- Consider vorticity confinement or vortex particles for local splash detail.

### The Granule-In-Cell Method for Simulating Sand-Water Mixtures

Paper:
[The Granule-In-Cell Method for Simulating Sand-Water Mixtures](https://arxiv.org/abs/2504.00745)

What it is:

This paper couples DEM granules with PIC-style fluid representation for
sand-water mixtures.

Like you are 12:

It treats sand grains as little objects but lets the water live in a smoother
field. The hard part is making them exchange pushes without losing mass.

Why it is interesting:

- It targets sand-water interaction directly.
- It handles migration, deposition, plugging, and dam-break behavior.
- It treats granules as macroscopic transport flow instead of just solid
  obstacles.

Why it matters for water:

- This is relevant if you want voxel/falling-sand-style water plus particles.
- It suggests a path for beaches, muddy water, sediment, and destructible terrain.

Practical takeaway:

- Sand-water is a separate system, not just "water with collision."
- Start with one-way coupling, then add two-way coupling if the project needs it.

### OceanSim: GPU-Accelerated Underwater Robot Perception Simulation

Paper:
[OceanSim paper](https://arxiv.org/abs/2503.01074)

Project page/demo/code:
[OceanSim project page](https://umfieldrobotics.github.io/OceanSim/)

What it is:

OceanSim is an underwater simulation framework focused on perception: visual and
acoustic sensors, underwater rendering, and real-time synthetic data generation.

Like you are 12:

This is less about making waves splash and more about making a robot's underwater
camera/sonar see the world like it would in real water.

Why it is interesting:

- It focuses on physics-based underwater rendering.
- It includes imaging sonar rendering.
- It targets GPU acceleration and real-time synthetic data.

Why it matters for water:

- If your simulator includes underwater views, this area matters.
- Underwater realism is about scattering, attenuation, haze, particles, and
  sensor effects, not just surface waves.

Practical takeaway:

- Underwater rendering should be a separate renderer mode with absorption,
  scattering, caustics, suspended particles, and volumetric light shafts.

### An Adjoint Method for Differentiable Fluid Simulation on Flow Maps

Paper:
[An Adjoint Method for Differentiable Fluid Simulation on Flow Maps](https://arxiv.org/abs/2511.01259)

What it is:

An adjoint method computes gradients backward through a simulation. This paper
does that using flow maps, so it can optimize/control fluids without storing
every intermediate simulation step.

Like you are 12:

If a splash ended up wrong, this method helps answer, "which earlier push should
I change to make the splash land where I want?"

Why it is interesting:

- It connects flow maps to differentiable simulation.
- It targets low memory for long-range gradient tracking.
- It supports optimization/control tasks.

Why it matters for water:

- Not a first real-time feature.
- Very relevant for automatic tuning, inverse design, and training learned
  components.

Practical takeaway:

- Keep simulation parameters explicit and structured. Later you may want to
  optimize them automatically.

## 2024

### Eulerian-Lagrangian Fluid Simulation on Particle Flow Maps

Paper:
[Eulerian-Lagrangian Fluid Simulation on Particle Flow Maps](https://arxiv.org/abs/2405.09672)

What it is:

This replaces the neural flow-map representation with particle flow maps. It
keeps the flow-map benefits but reports far lower time and memory costs than the
2023 neural version.

Like you are 12:

The 2023 version used tiny neural networks to remember motion. This one says:
"Wait, moving particles already remember motion. Use them."

Why it is interesting:

- It uses Lagrangian particles as natural flow-map carriers.
- It uses a background grid for incompressibility.
- It reports major speed and memory improvements over Neural Flow Maps.

Why it matters for water:

- Hybrid particle-grid thinking is central to FLIP/APIC and many water solvers.
- Better advection means better vortices and less smearing.

Practical takeaway:

- For advanced water, particles do not have to be only mass points. They can also
  store history, maps, vorticity, level-set data, foam age, and other rich state.

### Gaussian Splashing

Paper:
[Gaussian Splashing: Unified Particles for Versatile Motion Synthesis and Rendering](https://arxiv.org/abs/2401.15318)

Project/demo:
[Gaussian Splashing project page](https://gaussiansplashing.github.io/)

What it is:

Gaussian Splashing uses Gaussian particles as a unified representation for
motion synthesis and rendering.

Like you are 12:

Instead of turning everything into triangles, imagine every moving bit is a soft
little glowing blob. Many blobs together can look like a surface, splash, or
volume.

Why it is interesting:

- It sits between particles and rendering.
- It hints at alternatives to mesh-first rendering.
- It is relevant to splashes, droplets, and volumetric effects.

Why it matters for water:

- Whitewater, mist, and spray are particle-heavy effects.
- Gaussian/splat representations may be useful for rendering high-count droplets
  and foam without meshing everything.

Practical takeaway:

- Keep an eye on splat-based rendering for spray/mist.
- For WebGPU, instanced billboard/splat particles are a practical cousin of this
  idea.

### Symmetric Basis Convolutions for Learning Lagrangian Fluid Mechanics

Paper:
[Symmetric Basis Convolutions for Learning Lagrangian Fluid Mechanics](https://arxiv.org/abs/2403.16680)

Code/data:
[SFBC GitHub](https://github.com/tum-pbs/SFBC)

What it is:

This paper studies neural architectures for learning Lagrangian fluid mechanics,
especially SPH-like particle dynamics.

Like you are 12:

Particles do not care if you rotate the whole world; the physics should still be
the same. This paper builds that kind of symmetry into the learning method.

Why it is interesting:

- It evaluates basis functions for continuous convolutions.
- It shows symmetry matters for stability and accuracy.
- It tests compressible and incompressible SPH-like settings.

Why it matters for water:

- If you later learn a correction model for SPH/PBF particles, symmetry is not
  optional. A learned water model that changes behavior when rotated is broken.

Practical takeaway:

- Use learned particle models only if they respect physical symmetries:
  translation, rotation, locality, and conservation-like behavior.

### GeoFlood: Quadtree Flood Simulation

Paper:
[GeoFlood: Computational model for overland flooding](https://arxiv.org/abs/2403.15435)

What it is:

GeoFlood solves shallow-water equations on a quadtree hierarchy of mapped grids.
It is aimed at large-scale flooding rather than game water.

Like you are 12:

A flood over a city does not need tiny 3D splashes everywhere. It needs a smart
2D water sheet that gets more detailed near streets, walls, and fast changes.

Why it is interesting:

- Uses quadtree adaptive resolution.
- Targets large-scale flood waves on complex terrain.
- Validates against benchmarks and historical dam failure.

Why it matters for water:

- Shoreline/rivers/flooding can use shallow-water LOD instead of full 3D fluid.
- This is relevant to open-world water and terrain interaction.

Practical takeaway:

- For rivers, floods, and shoreline wash, consider a 2D shallow-water layer with
  adaptive tiles. Save 3D particles for splashes and breaking details.

### JAX-Fluids 2.0

Paper:
[JAX-Fluids 2.0](https://arxiv.org/abs/2402.05193)

Code:
[JAX-Fluids GitHub](https://github.com/tumaer/JAXFLUIDS)

What it is:

A differentiable CFD solver framework with HPC scaling and two-phase flow
support.

Like you are 12:

It is a serious science simulator that also lets machine learning see "which
input caused which output."

Why it is interesting:

- Differentiable.
- GPU/TPU/HPC-oriented.
- Includes compressible and two-phase flow modeling.

Why it matters for water:

- It is not a direct WebGPU game-water solution.
- It shows where research tooling is going: differentiable, high-order,
  accelerator-friendly fluid solvers.

Practical takeaway:

- For your project, differentiability is probably a later offline tool, not a
  real-time runtime feature.

### Droplet Simulation Survey

Paper:
[Droplet Simulations in Computer Graphics: Theories, Methods and Applications](https://arxiv.org/abs/2411.15880)

What it is:

A survey of droplet simulation methods in computer graphics, including particle
methods such as PBD and SPH.

Like you are 12:

It is a map of how people simulate little splashes and drops.

Why it matters for water:

- Droplets are a key part of spray.
- Surface tension, wall interaction, and small-scale particles are hard.

Practical takeaway:

- Treat droplets/spray as their own simulation/rendering layer. Do not expect
  the main water body solver to produce every visible droplet.

## 2023

### Fluid Simulation on Neural Flow Maps

Paper:
[Fluid Simulation on Neural Flow Maps](https://arxiv.org/abs/2312.14635)

What it is:

This paper uses implicit neural representations to store long-term velocity
fields and flow maps, improving advection and preserving vortical structures.

Like you are 12:

The neural network acts like a compressed memory of how the fluid moved, so the
sim can move details around without smearing them as much.

Why it is interesting:

- It is one of the clearest recent "new direction" papers in fluid simulation.
- It combines neural fields, sparse multiresolution grids, and flow maps.
- It targets energy conservation and complex vortex preservation.

Why it matters for water:

- Vortical detail drives turbulent splashes and whitewater.
- The exact method may be too heavy for your first real-time project, but the
  principle matters: advection quality is a bottleneck.

Practical takeaway:

- When testing solvers, do not only ask "does it move?" Ask "does it preserve
  swirl/detail over time?"

### FluidLab

Paper:
[FluidLab: A Differentiable Environment for Benchmarking Complex Fluid Manipulation](https://arxiv.org/abs/2303.02346)

Project/code:
[FluidLab GitHub](https://github.com/zhouxian/FluidLab)

What it is:

FluidLab is a differentiable simulation environment for complex fluid
manipulation tasks, including solid-fluid and multi-fluid interactions.

Like you are 12:

It is a training gym where robots can practice moving fluids, and the simulator
can tell learning algorithms how to improve.

Why it is interesting:

- Built around GPU-accelerated differentiable simulation.
- Includes complex material behavior and multi-component interactions.
- Useful for control and optimization.

Why it matters for water:

- If you ever want interactive tools, design optimization, or learning-based
  control, differentiable simulation becomes valuable.
- It is also relevant to non-water materials such as mud, cream, and granular
  mixtures.

Practical takeaway:

- For a water game/demo, do not start here. But borrow the idea that different
  fluid-like materials need different models.

### Wavelet Based Modeling of Subgrid Scales in LES of Particle-Laden Turbulent Flows

Paper:
[Wavelet based modeling of subgrid-scales in LES of particle-laden turbulent flows](https://arxiv.org/abs/2305.09521)

What it is:

This paper uses divergence-free wavelet vector bases to reconstruct subgrid-scale
velocity for large-eddy simulation of particle-laden turbulence.

Like you are 12:

The sim calculates the big swirls. The wavelet model adds back the little swirls
that were too small to calculate directly.

Why it is interesting:

- It uses wavelets to represent missing turbulence.
- It improves particle statistics and particle-pair dispersion.
- It keeps computational cost around LES scale rather than DNS scale.

Why it matters for water:

- Spray and mist are particle-laden turbulent flows in visual terms.
- This gives a principled version of "fake the tiny turbulent motion."

Practical takeaway:

- For whitewater, drive particle motion with a layered velocity field:
  base solver velocity plus procedural/wavelet-like turbulent detail.

### SURFSUP: Learning Fluid Simulation for Novel Surfaces

Paper:
[SURFSUP: Learning Fluid Simulation for Novel Surfaces](https://arxiv.org/abs/2304.06197)

Project:
[SURFSUP project page](https://arijitray1993.github.io/SURFSUP/)

What it is:

SURFSUP learns fluid-object interaction using SDF object representations so it
can generalize to new surfaces.

Like you are 12:

Instead of memorizing one bowl or one wall, it learns how fluid behaves near
shapes described by "how far am I from the surface?"

Why it is interesting:

- Uses signed distance fields for geometry.
- Targets generalization to novel surfaces.
- Supports inverse design of simple objects to manipulate flow.

Why it matters for water:

- Collisions with arbitrary geometry are a major problem for water.
- SDFs are also practical in WebGPU for collision and raymarching.

Practical takeaway:

- Store collision geometry as SDFs or proxy fields where possible. It helps
  particles, raymarching, and potentially learned correction models.

### GPU-Based Hydrodynamic Simulator with Boid Interactions

Paper:
[A GPU-based Hydrodynamic Simulator with Boid Interactions](https://arxiv.org/abs/2311.15088)

What it is:

A GPU SPH system with boid/agent interaction and real-time marching-cubes
surface reconstruction.

Like you are 12:

It puts little swimming agents inside particle water, lets them push the water,
and builds a triangle surface from the particles every frame.

Why it is interesting:

- Directly relevant to real-time SPH.
- Uses GPU compute.
- Includes per-frame marching cubes from particle data.

Why it matters for water:

- This is close to the architecture you were considering: particles plus GPU mesh
  reconstruction.

Practical takeaway:

- Per-frame marching cubes is possible, but it is compute/memory heavy. Start
  with screen-space fluid rendering unless you specifically need mesh geometry.

## 2022

### Water Simulation and Rendering from a Still Photograph

Paper:
[Water Simulation and Rendering from a Still Photograph](https://arxiv.org/abs/2210.02553)

What it is:

This paper animates realistic water from a single photograph using neural
segmentation/estimation plus screen-space local reflection rendering.

Like you are 12:

It looks at a still photo of a lake and guesses enough about the water to make it
move.

Why it is interesting:

- It focuses on perception and rendering, not just simulation.
- It estimates water masks, parameters, and reflection textures.
- It generates real-time water animation from image input.

Why it matters for water:

- It reinforces that believable water is heavily image/perception-driven.
- Reflection modeling and parameter estimation are as important as raw physics.

Practical takeaway:

- Build visual debugging views for reflection, refraction, depth color, foam, and
  normals. The renderer is not a decorative afterthought.

### Large Eddy Simulations of Bubbly Flows and Breaking Waves with SPH

Paper:
[Large Eddy Simulations of bubbly flows and breaking waves with SPH](https://arxiv.org/abs/2206.01641)

What it is:

This couples SPH with a discrete bubble model for breaking waves and bubbly
flows, using sub-resolution closures for bubbles.

Like you are 12:

Instead of simulating every tiny bubble perfectly, it simulates the big water
motion and uses a smart bubble model for the tiny stuff.

Why it is interesting:

- Bubbles are treated as Lagrangian particles.
- The method models bubble breakup, entrainment, and interaction with the free
  surface.
- It targets breaking waves, which are central to whitewater.

Why it matters for water:

- Whitewater is air-water mixture, not just white paint.
- Bubble distributions influence foam, underwater haze, and wave impact.

Practical takeaway:

- Model bubbles/spray/foam as secondary particles with their own rules.
- Let the main fluid emit them based on breaking/impact/turbulence signals.

### Learned Turbulence Modelling with Differentiable Fluid Solvers

Paper:
[Learned Turbulence Modelling with Differentiable Fluid Solvers](https://arxiv.org/abs/2202.06988)

Code/data:
[GitHub link from paper](https://github.com/tum-pbs/Solver-in-the-Loop)

What it is:

This trains turbulence models using differentiable fluid solvers and physics-based
losses.

Like you are 12:

The low-res sim misses tiny swirls. A learned model guesses what those swirls
should do, but it is trained by watching the sim over many future steps, not just
one frame.

Why it is interesting:

- Uses solver-in-the-loop training.
- Improves long-term turbulence statistics.
- Shows why multi-step training matters for stability.

Why it matters for water:

- Turbulence/detail correction is a promising learned component.
- For games, learned small-scale detail may be safer than learned whole-fluid
  dynamics.

Practical takeaway:

- If using ML, start with learned subgrid/detail correction, not a full learned
  water engine.

### Fluid Simulation System Based on Graph Neural Network

Paper:
[Fluid Simulation System Based on Graph Neural Network](https://arxiv.org/abs/2202.12619)

What it is:

A graph neural network simulator for flow fields.

Like you are 12:

A graph is a bunch of dots connected by lines. The model learns how each dot
should update by talking to nearby dots.

Why it is interesting:

- Graphs can adapt to irregular domains.
- The paper reports large speedups over traditional CFD for its target setup.

Why it matters for water:

- Graph methods are relevant for irregular geometry and adaptive sampling.
- They are not yet a drop-in game-water solution.

Practical takeaway:

- Watch graph neural solvers, but do not make them your first core simulator.

## 2021

### Learning Incompressible Fluid Dynamics from Scratch

Paper:
[Learning Incompressible Fluid Dynamics from Scratch](https://arxiv.org/abs/2006.08762)

What it is:

This was submitted in 2020 and published at ICLR 2021. It trains neural models
for incompressible fluid dynamics using physics-constrained training without
requiring simulation data.

Like you are 12:

Instead of showing the neural network thousands of example simulations, the
training process tells it the rules of fluid motion and lets it learn from those
rules.

Why it is interesting:

- Generalization was a major goal.
- It provides a differentiable update step.
- It includes interactive real-time demo material in the arXiv ancillary files.

Why it matters for water:

- It is part of the shift toward neural/differentiable fluids.
- It is more relevant to solver acceleration and control than immediate
  photoreal water rendering.

Practical takeaway:

- The safe near-term use of learned fluids is assistance: parameter prediction,
  upres, subgrid turbulence, or control.

### SuperCaustics

Paper:
[SuperCaustics: Real-time, open-source simulation of transparent objects for deep learning applications](https://arxiv.org/abs/2107.11008)

What it is:

SuperCaustics is a real-time open-source system for transparent-object simulation
with refraction, dispersion, and caustics, aimed at synthetic data generation.

Like you are 12:

It makes fake camera images of glassy things with realistic bent light and bright
caustic patterns.

Why it is interesting:

- Uses hardware ray tracing.
- Supports caustics/refraction/dispersion.
- Makes synthetic datasets with pixel-perfect labels.

Why it matters for water:

- Water is transparent and refractive.
- Caustics and refraction are central visual cues.
- Even though the target is computer vision, the rendering problems overlap with
  water.

Practical takeaway:

- On WebGPU, use projected caustics first.
- On Vulkan/desktop, hardware ray tracing can become useful for transparent
  optical effects, but it should not be the first milestone.

### Neural Geometric Level of Detail

Paper:
[Neural Geometric Level of Detail: Real-time Rendering with Implicit 3D Shapes](https://arxiv.org/abs/2101.10994)

What it is:

This paper uses an octree-based feature volume for real-time rendering of neural
SDFs with level of detail.

Like you are 12:

It stores a 3D shape like a smart block tower: tiny blocks near important detail,
big blocks where nothing interesting happens.

Why it is interesting:

- Octree-based neural representation.
- Continuous LOD.
- Real-time implicit shape rendering.

Why it matters for water:

- Not a water paper, but relevant to raymarched/SDF water surfaces.
- It connects adaptive spatial structures with real-time implicit rendering.

Practical takeaway:

- SDF/raymarch water should eventually use sparse/adaptive structures, not dense
  full-volume grids.

## 2020

### Adaptive Central-Upwind Scheme on Quadtree Grids for Variable Density Shallow Water Equations

Paper:
[An adaptive central-upwind scheme on quadtree grids for variable density shallow water equations](https://arxiv.org/abs/2008.02111)

What it is:

This is an adaptive quadtree method for shallow-water equations with variable
density.

Like you are 12:

The water surface is treated like a sheet. The sheet gets small squares where
interesting things happen and big squares where it is calm.

Why it is interesting:

- Uses quadtree refinement/coarsening.
- Designed to preserve stable water depth and density.
- Relevant to hydrodynamics and large domains.

Why it matters for water:

- Rivers, floods, shorelines, and wake maps can often be 2D shallow-water
  problems.
- Adaptive grids are a key LOD concept.

Practical takeaway:

- A WebGPU water engine could use tiled adaptive height fields for rivers and
  shorelines before attempting full 3D fluid everywhere.

### The Turbulent Bubble Break-Up Cascade: Numerical Simulations of Breaking Waves

Paper:
[The turbulent bubble break-up cascade. Part 2. Numerical simulations of breaking waves](https://arxiv.org/abs/2009.04804)

What it is:

This studies bubble-size distributions in breaking waves.

Like you are 12:

When a wave crashes, it traps big air pockets. Those pockets break into smaller
and smaller bubbles. The pattern of bubble sizes is not random noise; it follows
rules.

Why it is interesting:

- It examines how bubbles break up over time.
- It connects breaking waves to bubble population statistics.
- It helps explain real whitewater behavior.

Why it matters for water:

- Foam is made from air mixed into water.
- Realistic whitewater should consider bubble size/lifetime, not just a white
  texture.

Practical takeaway:

- Use different whitewater particle classes: large bubbles, small bubbles, foam,
  spray, and mist. Give them different lifetimes and render styles.

### Perceptual Evaluation of Liquid Simulation Methods

Paper:
[Perceptual Evaluation of Liquid Simulation Methods](https://arxiv.org/abs/2011.10257)

What it is:

This paper evaluates liquid simulation methods by asking people to compare
simulation videos against real references.

Like you are 12:

Instead of only asking "is the math correct?", it asks "does this look real to
people?"

Why it is interesting:

- It frames visual accuracy as a user-perception problem.
- It shows that reference videos make evaluations more consistent.
- It is a reminder that better physics does not always mean better perceived
  realism.

Why it matters for water:

- Your goal is visual real-time water. Perception matters.
- Foam, spray, lighting, and scale cues may matter more than small solver errors.

Practical takeaway:

- Build side-by-side reference tests early. Compare your sim to real footage for
  splashes, wakes, foam, and caustics.

### Efficient 2D Simulation on Moving 3D Surfaces

Paper:
[Efficient 2D Simulation on Moving 3D Surfaces](https://arxiv.org/abs/2009.00408)

What it is:

This simulates 2D flow on moving 3D surfaces, such as material flow over a water
surface or oil film effects.

Like you are 12:

Sometimes you do not need to simulate a whole new 3D fluid. You just need a thin
layer sliding on top of another moving surface.

Why it is interesting:

- Adds high-resolution 2D simulations on coarse animated surfaces.
- Uses sparse volume data and closest point methods.
- Useful for surface films and secondary surface effects.

Why it matters for water:

- Foam, oil, wetness, and floating scum can be treated as surface layers.
- This is a good mental model for foam maps on top of ocean waves.

Practical takeaway:

- Treat foam as a surface simulation layer when possible, not only as particles.

## Older Context Still Worth Knowing

These are outside the requested 2020-2026 window but directly explain techniques
you mentioned.

### Water Surface Wavelets

Reference:
[Macklin publications page including Water Surface Wavelets](https://blog.mmacklin.com/publications/)

Why it matters:

Water surface wavelets are a local wave representation. They are useful when you
want localized ripples and wakes rather than a global FFT ocean.

Like you are 12:

FFT waves are like ocean-wide music. A wavelet is a little pluck at one place on
the water.

How it connects to newer work:

- The 2025 hybrid FFT/wave-particle ocean paper is spiritually similar: global
  spectrum plus local wave detail.
- Wavelet/subgrid turbulence papers continue the idea of localized, scale-aware
  detail.

### Wavelet Turbulence

Reference:
[Wavelet Turbulence for Fluid Simulation](https://www.cs.cornell.edu/~tedkim/wturb/)

Why it matters:

Wavelet turbulence is the classic graphics trick of adding high-frequency
turbulent detail to lower-resolution simulations.

How it connects to water:

- Mist, spray, and foam need small-scale chaotic motion.
- Simulating all of that physically is too expensive.

### Tessendorf / FFT Oceans

Reference:
[Tessendorf, Simulating Ocean Water](https://people.computing.clemson.edu/~jtessen/reports/papers_files/coursenotes2004.pdf)

Why it matters:

This is still the root of many production-style spectral ocean systems.

How it connects to newer work:

- Modern hybrid ocean papers still build on the FFT/spectral ocean idea.
- The new work is about interaction, coupling, LOD, and local patches.

## What I Would Actually Use in This Project

### Near-Term WebGPU Path

Use these ideas now:

- FFT/Gerstner ocean base.
- Wave-particle or wake-map local interaction.
- Projected caustics.
- Foam maps.
- Spray/mist particles.
- Screen-space fluid rendering for small local PBF/SPH experiments.
- SDF collision proxies.
- Tiled/adaptive data structures, even if simple.

Do not start with:

- Full neural solver.
- Full flow-map level set.
- Hardware ray tracing.
- Full sand-water DEM/PIC coupling.

Why:

The recent papers are exciting, but most are research-heavy. Your first high-end
demo should turn recent ideas into practical approximations.

### Medium-Term Advanced Path

Promising upgrades:

- Local 3D PBF/SPH/DFSPH patch.
- Vorticity-driven whitewater emission.
- Wavelet/procedural turbulence for spray/mist.
- Shallow-water adaptive tile solver for rivers/shorelines.
- Hybrid FFT + wave-particle ocean coupling.
- SDF or sparse volume surface rendering.

### Long-Term Desktop/Vulkan Path

Research-heavy upgrades:

- Particle flow-map-inspired advection.
- Level-set surface tracking for high-quality local water.
- Vulkan hardware ray tracing for reflection/refraction experiments.
- GPU marching cubes for hero water patches.
- Learned turbulence/detail correction.
- Differentiable/offline tools to tune parameters.

## Most Important Lessons

1. Hybrid is the winning pattern.

Do not force one solver to handle ocean swell, boat wakes, buckets, bubbles,
spray, and shoreline flooding.

2. Local detail is the new frontier.

FFT oceans are mature. The recent excitement is about local interaction,
adaptive resolution, flow maps, whitewater, and perceptual/rendering quality.

3. Wavelets are about localized detail.

Use wavelet thinking for turbulence, foam, spray, and local ripples: detail
should appear where energy exists, not everywhere.

4. Quadtrees/octrees are about spending resolution wisely.

Use adaptive grids/tiles for shorelines, wakes, sparse volumes, and SDF/raymarch
structures.

5. Whitewater deserves its own system.

The bubble/breaking-wave papers reinforce that foam and bubbles are not just
surface color. They are air-water mixture, with lifetime, size, buoyancy, and
scattering behavior.

6. Neural methods are promising but not yet your core.

Use ML later for upres, denoising, parameter tuning, subgrid turbulence, or
learned corrections. Start with explicit simulation/rendering you can debug.

7. Rendering is half the battle.

Water realism depends on optical effects: reflection, refraction, caustics,
scattering, underwater haze, foam lighting, and spray shading. Some of the most
useful "water simulation" papers are really rendering/perception papers.

## Link Index

2026:

- [A Level Set Method on Particle Flow Maps](https://arxiv.org/abs/2601.09939)
- [Adaptive GPU Kinetic Solver for Fluid-Granular Flows](https://arxiv.org/abs/2603.14982)

2025:

- [Real-Time Interactive Hybrid Ocean: Spectrum-Consistent Wave Particle-FFT Coupling](https://arxiv.org/abs/2511.02852)
- [Arc Blanc: a real time ocean simulation framework](https://arxiv.org/abs/2503.03326)
- [Fluid Simulation on Vortex Particle Flow Maps](https://arxiv.org/abs/2505.21946)
- [The Granule-In-Cell Method for Simulating Sand-Water Mixtures](https://arxiv.org/abs/2504.00745)
- [OceanSim paper](https://arxiv.org/abs/2503.01074)
- [OceanSim project page](https://umfieldrobotics.github.io/OceanSim/)
- [An Adjoint Method for Differentiable Fluid Simulation on Flow Maps](https://arxiv.org/abs/2511.01259)

2024:

- [Eulerian-Lagrangian Fluid Simulation on Particle Flow Maps](https://arxiv.org/abs/2405.09672)
- [Gaussian Splashing paper](https://arxiv.org/abs/2401.15318)
- [Gaussian Splashing project page](https://gaussiansplashing.github.io/)
- [Symmetric Basis Convolutions for Learning Lagrangian Fluid Mechanics](https://arxiv.org/abs/2403.16680)
- [SFBC GitHub](https://github.com/tum-pbs/SFBC)
- [GeoFlood](https://arxiv.org/abs/2403.15435)
- [JAX-Fluids 2.0](https://arxiv.org/abs/2402.05193)
- [JAX-Fluids GitHub](https://github.com/tumaer/JAXFLUIDS)
- [Droplet Simulations in Computer Graphics](https://arxiv.org/abs/2411.15880)

2023:

- [Fluid Simulation on Neural Flow Maps](https://arxiv.org/abs/2312.14635)
- [FluidLab](https://arxiv.org/abs/2303.02346)
- [FluidLab GitHub](https://github.com/zhouxian/FluidLab)
- [Wavelet based modeling of subgrid-scales in LES of particle-laden turbulent flows](https://arxiv.org/abs/2305.09521)
- [SURFSUP](https://arxiv.org/abs/2304.06197)
- [SURFSUP project page](https://arijitray1993.github.io/SURFSUP/)
- [A GPU-based Hydrodynamic Simulator with Boid Interactions](https://arxiv.org/abs/2311.15088)

2022:

- [Water Simulation and Rendering from a Still Photograph](https://arxiv.org/abs/2210.02553)
- [Large Eddy Simulations of bubbly flows and breaking waves with SPH](https://arxiv.org/abs/2206.01641)
- [Learned Turbulence Modelling with Differentiable Fluid Solvers](https://arxiv.org/abs/2202.06988)
- [Solver-in-the-Loop GitHub](https://github.com/tum-pbs/Solver-in-the-Loop)
- [Fluid Simulation System Based on Graph Neural Network](https://arxiv.org/abs/2202.12619)

2021:

- [Learning Incompressible Fluid Dynamics from Scratch](https://arxiv.org/abs/2006.08762)
- [SuperCaustics](https://arxiv.org/abs/2107.11008)
- [Neural Geometric Level of Detail](https://arxiv.org/abs/2101.10994)

2020:

- [Adaptive central-upwind scheme on quadtree grids for variable density shallow water equations](https://arxiv.org/abs/2008.02111)
- [The turbulent bubble break-up cascade. Part 2](https://arxiv.org/abs/2009.04804)
- [Perceptual Evaluation of Liquid Simulation Methods](https://arxiv.org/abs/2011.10257)
- [Efficient 2D Simulation on Moving 3D Surfaces](https://arxiv.org/abs/2009.00408)

Older context:

- [Water Surface Wavelets listing](https://blog.mmacklin.com/publications/)
- [Wavelet Turbulence for Fluid Simulation](https://www.cs.cornell.edu/~tedkim/wturb/)
- [Tessendorf, Simulating Ocean Water](https://people.computing.clemson.edu/~jtessen/reports/papers_files/coursenotes2004.pdf)
