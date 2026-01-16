//! 3D Model Viewer plugin state types

use glam::{Mat4, Vec3};
use std::path::PathBuf;

/// Render mode for 3D model display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// Wireframe ASCII rendering
    #[default]
    Ascii,
    /// Rendered image (software rasterization)
    Image,
}

/// Draw style for 3D rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawStyle {
    /// Wireframe (edges only)
    #[default]
    Wireframe,
    /// Filled/solid triangles
    Filled,
}

/// Current view in the model plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelView {
    #[default]
    /// Main viewer
    Viewer,
    /// Error state
    Error,
}

/// A 3D vertex
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: Vec3,
}

/// A triangle face (indices into vertex array)
#[derive(Debug, Clone, Copy)]
pub struct Face {
    pub v0: usize,
    pub v1: usize,
    pub v2: usize,
}

/// A loaded 3D model
#[derive(Debug, Clone, Default)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
    pub center: Vec3,
    pub scale: f32,
}

impl Model {
    /// Calculate bounding box and normalize model to fit in unit cube
    pub fn normalize(&mut self) {
        if self.vertices.is_empty() {
            return;
        }

        // Find bounding box
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for v in &self.vertices {
            min = min.min(v.position);
            max = max.max(v.position);
        }

        // Calculate center and scale
        self.center = (min + max) / 2.0;
        let size = max - min;
        self.scale = size.x.max(size.y).max(size.z);

        if self.scale > 0.0 {
            // Center and scale vertices
            for v in &mut self.vertices {
                v.position = (v.position - self.center) / self.scale;
            }
            self.center = Vec3::ZERO;
            self.scale = 1.0;
        }
    }
}

/// Camera state
#[derive(Debug, Clone)]
pub struct Camera {
    pub distance: f32,
    pub rotation_x: f32, // Pitch
    pub rotation_y: f32, // Yaw
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            distance: 3.0,
            rotation_x: 0.3, // Slight downward angle
            rotation_y: 0.0,
        }
    }
}

impl Camera {
    /// Get view matrix
    pub fn view_matrix(&self) -> Mat4 {
        let eye = Vec3::new(
            self.distance * self.rotation_y.cos() * self.rotation_x.cos(),
            self.distance * self.rotation_x.sin(),
            self.distance * self.rotation_y.sin() * self.rotation_x.cos(),
        );
        Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y)
    }
}

/// Model viewer state
#[derive(Debug, Default)]
pub struct ModelState {
    /// Current view
    pub view: ModelView,
    /// Loaded model
    pub model: Option<Model>,
    /// Camera state
    pub camera: Camera,
    /// File path
    pub file_path: Option<PathBuf>,
    /// File name for display
    pub file_name: String,
    /// Error message
    pub error: Option<String>,
    /// Render mode (ASCII vs Image)
    pub render_mode: RenderMode,
    /// Draw style (Wireframe vs Filled)
    pub draw_style: DrawStyle,
    /// Auto-rotate enabled
    pub auto_rotate: bool,
    /// Sibling model files
    pub sibling_files: Vec<PathBuf>,
    /// Current file index
    pub current_file_index: usize,
}

impl ModelState {
    pub fn new() -> Self {
        Self {
            auto_rotate: true,
            ..Default::default()
        }
    }

    /// Toggle render mode (ASCII vs Image)
    pub fn toggle_render_mode(&mut self) {
        self.render_mode = match self.render_mode {
            RenderMode::Ascii => RenderMode::Image,
            RenderMode::Image => RenderMode::Ascii,
        };
    }

    /// Toggle draw style (Wireframe vs Filled)
    pub fn toggle_draw_style(&mut self) {
        self.draw_style = match self.draw_style {
            DrawStyle::Wireframe => DrawStyle::Filled,
            DrawStyle::Filled => DrawStyle::Wireframe,
        };
    }

    /// Detect sibling model files
    pub fn detect_siblings(&mut self) {
        let Some(ref file_path) = self.file_path else {
            return;
        };

        let Some(parent) = file_path.parent() else {
            return;
        };

        let mut siblings: Vec<PathBuf> = std::fs::read_dir(parent)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_model_file(p))
            .collect();

        siblings.sort();
        self.current_file_index = siblings.iter().position(|p| p == file_path).unwrap_or(0);
        self.sibling_files = siblings;
    }

    /// Check if there's a previous file
    pub fn has_prev(&self) -> bool {
        self.current_file_index > 0
    }

    /// Check if there's a next file
    pub fn has_next(&self) -> bool {
        !self.sibling_files.is_empty() && self.current_file_index < self.sibling_files.len() - 1
    }

    /// Get previous file path
    pub fn prev_file(&self) -> Option<PathBuf> {
        if self.has_prev() {
            self.sibling_files.get(self.current_file_index - 1).cloned()
        } else {
            None
        }
    }

    /// Get next file path
    pub fn next_file(&self) -> Option<PathBuf> {
        if self.has_next() {
            self.sibling_files.get(self.current_file_index + 1).cloned()
        } else {
            None
        }
    }

    /// Get file position string
    pub fn file_position(&self) -> String {
        if self.sibling_files.is_empty() {
            String::new()
        } else {
            format!(
                "{}/{}",
                self.current_file_index + 1,
                self.sibling_files.len()
            )
        }
    }
}

/// Check if a file is a 3D model file
pub fn is_model_file(path: &PathBuf) -> bool {
    path.extension()
        .map(|ext| {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            matches!(ext_lower.as_str(), "obj" | "stl")
        })
        .unwrap_or(false)
}
