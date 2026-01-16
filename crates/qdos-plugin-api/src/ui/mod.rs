//! UI Components for QDOS Plugins
//!
//! This module provides reusable UI components for plugin development.
//! These components use the plugin API's ThemeColors for consistent styling.

mod modal;
mod screen;

pub use modal::ModalFrame;
pub use screen::FullScreenView;
