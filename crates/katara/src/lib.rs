#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

pub mod app;
mod camera;
mod config;
mod debug_watch;
#[cfg(feature = "asset-loading")]
mod demo_scene;
mod frame_clock;
mod gui;
mod inspect;
mod profiling;
mod render;
mod scene;
mod state;
mod stats;
mod util;
