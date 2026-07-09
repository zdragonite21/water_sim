# CPU SPH Pipeline Restructure Plan

## Summary

Restructure the current water implementation so the existing CPU SPH particle sim becomes the first concrete `WaterPipeline`, without adding MLS-MPM, GPU compute, raymarching, or marching-cubes implementations yet.

The app-facing shape remains: `State -> WaterScene -> active water pipeline`. The CPU SPH method owns its sim, particle renderer, and CPU-specific debug overlay. Shared water infrastructure only keeps lifecycle coordination and reusable low-level debug drawing.

## Key Changes

- Complete the in-progress move instead of reverting it:
  - Keep `src/water/pipelines/cpu_sph_particles/` as the home for the current CPU SPH method.
  - Keep `src/water/debug/` for reusable line-debug primitives only.
  - Update `src/water/mod.rs`, `src/water/debug/mod.rs`, and `src/water/pipelines/mod.rs` so module paths compile cleanly.

- Rename method-specific types so future methods do not inherit generic names:
  - `WaterSim` -> `CpuSphSim`
  - `Particle` -> `CpuSphParticle`
  - `WaterSimStats` -> `CpuSphStats`
  - `WaterRenderer` -> `ParticleBillboardRenderer`
  - `DebugOverlay` -> `CpuSphDebugOverlay`
  - Add `CpuSphParticlesPipeline` as the owner of those three pieces.

- Add a scalable pipeline wrapper with one variant for now:
  - `WaterPipelineKind::CpuSphParticles`
  - `WaterPipeline::CpuSphParticles(CpuSphParticlesPipeline)`
  - Implement wrapper methods for `new`, `resize`, `reset`, `update_fixed`, `upload_frame`, `render`, `update_config`, `current_config`, and `stats`.
  - Use an enum, not a trait object, so future highly customized GPU/MPM pipelines are not forced through a fake-common interface.

- Change `WaterScene` to own only:
  - `pipeline: WaterPipeline`
  - pause/fixed-step timing state
  - scene-level lifecycle methods
  - no direct `sim`, `renderer`, or CPU-SPH debug overlay fields.

## Config, GUI, And Debug

- Convert water config from generic sim/render/debug fields into a pipeline-specific config:
  - `WaterConfig { active_pipeline, cpu_sph_particles }`
  - `CpuSphParticlesConfig { sim, render, debug }`
  - Rename current config types to method-specific names:
    - `WaterSimConfig` -> `CpuSphSimConfig`
    - `WaterRenderConfig` -> `ParticleBillboardConfig`
    - `DebugOverlayConfig` -> `CpuSphDebugConfig`

- Preserve current config loading:
  - Existing `[water.sim]`, `[water.render]`, and `[water.debug]` TOML should still load into `cpu_sph_particles`.
  - Saving after the restructure may write the new nested shape:
    - `[water] active_pipeline = "cpu_sph_particles"`
    - `[water.cpu_sph_particles.sim]`
    - `[water.cpu_sph_particles.render]`
    - `[water.cpu_sph_particles.debug]`

- Update the Water GUI panel:
  - Show the active pipeline as `CPU SPH Particles`.
  - Draw only `water_config.cpu_sph_particles`.
  - Use an ImGui ID scope for the pipeline panel so repeated labels like `enabled`, `gravity`, or `debug` do not collide when more pipelines are added later.

- Keep global debug text outside the pipeline:
  - FPS and `debug_watch` stay global.
  - Pipeline stats become rows supplied by the active pipeline, labeled with the active pipeline name.
  - CPU SPH visual debug overlay moves inside `CpuSphParticlesPipeline`.
  - Shared line drawing stays in `src/water/debug/`.

## Test Plan

- Run `cargo check` and fix all moved-module/import errors.
- Run `cargo test` if existing tests are present.
- Verify the app still starts with the CPU SPH particle pipeline selected.
- Verify pause, step, reset, resize, and rendering behave the same as before.
- Verify GUI edits still affect sim/render/debug config live.
- Verify old `game_config.toml` shape loads without losing current values.
- Verify saved config uses the new pipeline-specific shape if config saving is triggered.
- Verify global debug text still shows FPS, debug watches, and CPU SPH stats rows.

## Assumptions And Defaults

- Do not implement MLS-MPM, GPU SPH, raymarching, marching cubes, or placeholder folders for them in this change.
- Keep the current CPU SPH behavior visually and numerically equivalent unless a compile fix requires a mechanical name/path change.
- Prefer enum-based dispatch for now; revisit traits only after at least two real pipelines exist.
- The current working tree already contains a partial file move, so the implementation should finish that move rather than restoring the old flat `src/water/*.rs` layout.
