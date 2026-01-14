// Allow specific clippy lints that would require significant refactoring
#![allow(clippy::ptr_arg)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::manual_strip)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::lines_filter_map_ok)]

//! R-DOS: A retro DOS-style file manager TUI
//!
//! This library exposes the core modules for integration testing.

pub mod app;
pub mod clipboard;
pub mod config;
pub mod errors;
pub mod event;
pub mod file_ops;
pub mod mcp;
pub mod plugins;
pub mod rg;
pub mod sound;
pub mod ui;
pub mod vfs;
pub mod watcher;
