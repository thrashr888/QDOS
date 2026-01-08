//! 3D Model Viewer plugin
//!
//! View 3D model files (OBJ, STL) with ASCII wireframe or image rendering.

mod modal;
mod render;
pub mod state;

use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{Face, Model, ModelState, ModelView, Vertex};
use std::any::Any;
use std::path::PathBuf;

/// 3D Model Viewer plugin
pub struct Model3dPlugin {
    pub state: ModelState,
}

impl Default for Model3dPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Model3dPlugin {
    pub fn new() -> Self {
        Self {
            state: ModelState::new(),
        }
    }

    /// Open the modal for a specific model file
    pub fn open_modal(&mut self, file_path: Option<&PathBuf>) {
        self.state = ModelState::new();
        self.state.file_path = file_path.cloned();
        self.state.error = None;

        if let Some(path) = file_path {
            self.state.file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            self.state.detect_siblings();
            self.load_model(path.clone());
        }

        self.state.view = ModelView::Viewer;
    }

    /// Load a 3D model from file
    fn load_model(&mut self, path: PathBuf) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let result = match ext.as_str() {
            "obj" => self.load_obj(&path),
            "stl" => self.load_stl(&path),
            _ => Err(format!("Unsupported format: {}", ext)),
        };

        match result {
            Ok(mut model) => {
                model.normalize();
                self.state.model = Some(model);
            }
            Err(e) => {
                self.state.error = Some(e);
                self.state.view = ModelView::Error;
            }
        }
    }

    /// Load OBJ file
    fn load_obj(&self, path: &PathBuf) -> Result<Model, String> {
        let (models, _materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)
            .map_err(|e| format!("Failed to load OBJ: {}", e))?;

        if models.is_empty() {
            return Err("No meshes found in OBJ file".to_string());
        }

        let mut vertices = Vec::new();
        let mut faces = Vec::new();

        for model in models {
            let mesh = model.mesh;
            let vertex_offset = vertices.len();

            // Load vertices
            for i in 0..mesh.positions.len() / 3 {
                vertices.push(Vertex {
                    position: glam::Vec3::new(
                        mesh.positions[i * 3],
                        mesh.positions[i * 3 + 1],
                        mesh.positions[i * 3 + 2],
                    ),
                });
            }

            // Load faces
            for i in 0..mesh.indices.len() / 3 {
                faces.push(Face {
                    v0: mesh.indices[i * 3] as usize + vertex_offset,
                    v1: mesh.indices[i * 3 + 1] as usize + vertex_offset,
                    v2: mesh.indices[i * 3 + 2] as usize + vertex_offset,
                });
            }
        }

        Ok(Model {
            vertices,
            faces,
            center: glam::Vec3::ZERO,
            scale: 1.0,
        })
    }

    /// Load STL file (binary or ASCII)
    fn load_stl(&self, path: &PathBuf) -> Result<Model, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("Failed to open STL: {}", e))?;
        let mut reader = std::io::BufReader::new(file);

        // Try to detect if binary or ASCII
        let mut header = [0u8; 80];
        use std::io::Read;
        reader
            .read_exact(&mut header)
            .map_err(|e| format!("Failed to read STL header: {}", e))?;

        // Check if ASCII (starts with "solid")
        let header_str = String::from_utf8_lossy(&header[..5]);
        if header_str.starts_with("solid") {
            // Might be ASCII, try to parse
            return self.load_stl_ascii(path);
        }

        // Binary STL
        self.load_stl_binary(path)
    }

    fn load_stl_binary(&self, path: &PathBuf) -> Result<Model, String> {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read STL: {}", e))?;

        if data.len() < 84 {
            return Err("STL file too small".to_string());
        }

        let num_triangles = u32::from_le_bytes([data[80], data[81], data[82], data[83]]) as usize;

        let mut vertices = Vec::new();
        let mut faces = Vec::new();

        let mut offset = 84;
        for _ in 0..num_triangles {
            if offset + 50 > data.len() {
                break;
            }

            // Skip normal (12 bytes)
            offset += 12;

            // Read 3 vertices
            let v0_idx = vertices.len();
            for _ in 0..3 {
                let x = f32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                let y = f32::from_le_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]);
                let z = f32::from_le_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]);
                vertices.push(Vertex {
                    position: glam::Vec3::new(x, y, z),
                });
                offset += 12;
            }

            faces.push(Face {
                v0: v0_idx,
                v1: v0_idx + 1,
                v2: v0_idx + 2,
            });

            // Skip attribute byte count
            offset += 2;
        }

        Ok(Model {
            vertices,
            faces,
            center: glam::Vec3::ZERO,
            scale: 1.0,
        })
    }

    fn load_stl_ascii(&self, path: &PathBuf) -> Result<Model, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read STL: {}", e))?;

        let mut vertices = Vec::new();
        let mut faces = Vec::new();

        let mut current_vertices: Vec<Vertex> = Vec::new();

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[0] == "vertex" {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    current_vertices.push(Vertex {
                        position: glam::Vec3::new(x, y, z),
                    });

                    if current_vertices.len() == 3 {
                        let v0_idx = vertices.len();
                        vertices.append(&mut current_vertices);
                        faces.push(Face {
                            v0: v0_idx,
                            v1: v0_idx + 1,
                            v2: v0_idx + 2,
                        });
                    }
                }
            }
        }

        if vertices.is_empty() {
            return Err("No vertices found in STL file".to_string());
        }

        Ok(Model {
            vertices,
            faces,
            center: glam::Vec3::ZERO,
            scale: 1.0,
        })
    }

    /// Switch to a different model file
    fn switch_to_file(&mut self, file_path: PathBuf) {
        let siblings = std::mem::take(&mut self.state.sibling_files);
        let new_index = siblings.iter().position(|p| p == &file_path).unwrap_or(0);
        let render_mode = self.state.render_mode;
        let draw_style = self.state.draw_style;
        let auto_rotate = self.state.auto_rotate;

        self.state = ModelState::new();
        self.state.render_mode = render_mode;
        self.state.draw_style = draw_style;
        self.state.auto_rotate = auto_rotate;
        self.state.file_path = Some(file_path.clone());
        self.state.file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.state.sibling_files = siblings;
        self.state.current_file_index = new_index;

        self.load_model(file_path);
        self.state.view = ModelView::Viewer;
    }

    /// Check if a file is a 3D model file
    pub fn is_model_file(path: &PathBuf) -> bool {
        state::is_model_file(path)
    }
}

impl Plugin for Model3dPlugin {
    fn id(&self) -> &str {
        "model3d"
    }

    fn name(&self) -> &str {
        "3D Model Viewer"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: false,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "3D Model".to_string(),
            key: '3',
            description: "View 3D model files".to_string(),
            priority: 38,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            ModelView::Viewer => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Left => {
                    self.state.camera.rotation_y -= 0.1;
                    self.state.auto_rotate = false;
                    KeyHandleResult::Handled
                }
                KeyCode::Right => {
                    self.state.camera.rotation_y += 0.1;
                    self.state.auto_rotate = false;
                    KeyHandleResult::Handled
                }
                KeyCode::Up => {
                    self.state.camera.rotation_x = (self.state.camera.rotation_x + 0.1).min(1.5);
                    self.state.auto_rotate = false;
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    self.state.camera.rotation_x = (self.state.camera.rotation_x - 0.1).max(-1.5);
                    self.state.auto_rotate = false;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    self.state.camera.distance = (self.state.camera.distance - 0.5).max(1.0);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('-') => {
                    self.state.camera.distance = (self.state.camera.distance + 0.5).min(20.0);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.state.auto_rotate = !self.state.auto_rotate;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    self.state.toggle_render_mode();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    self.state.toggle_draw_style();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('[') => {
                    if let Some(prev) = self.state.prev_file() {
                        self.switch_to_file(prev);
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char(']') => {
                    if let Some(next) = self.state.next_file() {
                        self.switch_to_file(next);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            ModelView::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => KeyHandleResult::CloseModal,
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_model_modal(frame, area, &self.state, colors);
    }

    fn tick(&mut self) {
        // Auto-rotate if enabled
        if self.state.auto_rotate && self.state.model.is_some() {
            self.state.camera.rotation_y += 0.05;
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "3D Model Viewer".to_string(),
            "".to_string(),
            "View 3D model files with ASCII wireframe".to_string(),
            "or image rendering.".to_string(),
            "".to_string(),
            "Supported formats:".to_string(),
            "  OBJ - Wavefront OBJ".to_string(),
            "  STL - Stereolithography".to_string(),
            "".to_string(),
            "Controls:".to_string(),
            "  Arrows - Rotate model".to_string(),
            "  +/-    - Zoom in/out".to_string(),
            "  R      - Toggle auto-rotate".to_string(),
            "  F      - Toggle wireframe/filled".to_string(),
            "  M      - Toggle ASCII/Image mode".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "3D Model".to_string(),
            description: "View 3D model files".to_string(),
            category: PluginCategory::Tools,
            key: '3',
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
