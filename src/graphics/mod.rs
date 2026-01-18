//! Graphics utilities for R-DOS
//!
//! Provides retro-style graphics rendering capabilities including:
//! - Wireframe 3D rendering (Star Wars / Battlezone style)
//! - CRT phosphor effects
//! - ASCII fallback rendering

pub mod wireframe;

#[allow(unused_imports)]
pub use wireframe::{
    apply_scanlines, render_wireframe, render_wireframe_colored, wireframe_to_ascii,
    WireframeModel, PHOSPHOR_DARK, PHOSPHOR_DIM, PHOSPHOR_GREEN,
};
