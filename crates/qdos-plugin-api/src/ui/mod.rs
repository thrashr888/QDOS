//! UI Components for QDOS Plugins
//!
//! This module provides reusable UI components for plugin development.
//! These components use the plugin API's ThemeColors for consistent styling.

mod modal;
mod screen;
mod tabs;

pub use modal::ModalFrame;
pub use screen::FullScreenView;
pub use tabs::{TabBar, TabState};
