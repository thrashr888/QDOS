# R-DOS Graphics Skill

Use this skill when implementing graphical features using sixel graphics in R-DOS. This includes pixel art editing (Q-PAINT), game graphics, splash screens, logos, and any visual content that needs true pixel-level rendering.

## Overview

R-DOS uses **sixel graphics** for pixel-perfect rendering in terminals that support it. Sixels provide:
- 6 vertical pixels per character cell
- Full 24-bit RGB color support
- Hardware-accelerated rendering in modern terminals

## Terminal Support

### Sixel Protocol (Primary for R-DOS)

**Full Support:**
- **xterm** - X11, Linux (original implementation)
- **contour** - OpenGL, Linux/macOS/Windows
- **foot** - Wayland, Linux (fast)
- **mintty** - Windows
- **mlterm** - X11, Linux (original sixel terminal)
- **RLogin** - Windows
- **wezterm** - OpenGL, Linux/macOS/Windows
- **yaft** - Framebuffer, Linux

**Partial/Experimental:**
- **iTerm2** - macOS (works with quirks)
- **MacTerm** - macOS
- **alacritty** - graphics branch only
- **VTE/gnome-terminal** - wip/sixels branch
- **konsole** - via graphics PR
- **darktile** - OpenGL, Linux
- **DomTerm** - JavaScript/Electron

### Kitty Protocol

- **kitty** - OpenGL, Linux/macOS
- **wezterm** - OpenGL, Linux/macOS/Windows
- **konsole** - X11/Wayland, Linux

### iTerm2 Protocol

- **iTerm2** - macOS
- **wezterm** - Linux/macOS/Windows
- **mintty** - Windows
- **konsole** - Linux
- **hterm** - Web browser

### Unsupported Terminals

- **Terminal.app** (macOS default) - No graphics support
- **Alacritty** (stable) - No graphics support
- **Windows Terminal** - No sixel support (yet)
- **GNOME Terminal** (stable) - No sixel in release builds

### Detection

R-DOS uses `ratatui-image` which auto-detects the best protocol:

```rust
let picker = Picker::from_query_stdio()
    .unwrap_or_else(|_| Picker::from_fontsize((8, 16)));
```

When graphics are unavailable, show a warning:
```
This feature requires a graphics-capable terminal.
Recommended: Kitty, WezTerm, foot, or iTerm2
```

### Terminal Feature Queries

For terminal authors, essential features for sixel:
- DA1 with sixel (4) capability
- OSC 11 query (background color)
- OSC 4 query (ANSI colors)
- XTWINOPS 14, 16, 18 (cell pixel dimensions)
- XTSMGRAPHICS query (sixel registers)
- DECSET 1070 (per-image palette)

## Architecture

### Key Crates

```toml
[dependencies]
# Terminal graphics protocol detection and rendering
ratatui-image = "3.0"
image = "0.25"

# Sixel encoding (for custom rendering)
sixel-rs = "0.4"  # or sixel-image for simpler API

# Audio feedback
rodio = "0.19"
```

### Protocol Detection

R-DOS already has image protocol detection in `src/plugins/viewer/mod.rs`:

```rust
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

// Lazy-initialized picker detects best protocol
static IMAGE_PICKER: OnceLock<Mutex<Picker>> = OnceLock::new();

fn get_image_picker() -> &'static Mutex<Picker> {
    IMAGE_PICKER.get_or_init(|| {
        let picker = Picker::from_query_stdio()
            .unwrap_or_else(|_| Picker::from_fontsize((8, 16)));
        Mutex::new(picker)
    })
}
```

### Sixel Basics

Each sixel character represents 6 vertical pixels:
- Characters `?` (0x3F) to `~` (0x7E) = 64 combinations
- Each bit (0-5) controls one pixel from top to bottom
- Example: `?` = all off, `~` = all on, `o` = bottom 4 on

### Coordinate Systems

```
Terminal Coordinates          Pixel Coordinates (at 1x zoom)
┌─────────────────────┐      ┌─────────────────────┐
│ (0,0)      (79,0)   │      │ (0,0)    (639,0)    │
│                     │      │                     │
│                     │  →   │                     │
│                     │      │                     │
│ (0,24)     (79,24)  │      │ (0,149)  (639,149)  │
└─────────────────────┘      └─────────────────────┘
80x25 chars                   640x150 sixel pixels
(8x6 pixels per char)         (typical sixel resolution)
```

For a standard 80x25 terminal with 8x16 pixel font:
- Sixel width: 80 * 8 = 640 pixels
- Sixel height: 25 * 6 = 150 sixel rows (but characters are taller)

## Q-PAINT Implementation

### Canvas Architecture

```rust
pub struct SixelCanvas {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// RGBA pixel data (width * height * 4)
    pub pixels: Vec<u8>,
    /// Current zoom level (1, 2, 4, 8, 16)
    pub zoom: u8,
    /// Viewport offset in pixels
    pub scroll_x: u32,
    pub scroll_y: u32,
}

impl SixelCanvas {
    /// Get pixel at (x, y)
    pub fn get_pixel(&self, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let idx = ((y * self.width + x) * 4) as usize;
        (self.pixels[idx], self.pixels[idx+1],
         self.pixels[idx+2], self.pixels[idx+3])
    }

    /// Set pixel at (x, y)
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: (u8, u8, u8, u8)) {
        let idx = ((y * self.width + x) * 4) as usize;
        self.pixels[idx..idx+4].copy_from_slice(&[rgba.0, rgba.1, rgba.2, rgba.3]);
    }
}
```

### Rendering Pipeline

1. **Edit pixels** in the canvas buffer
2. **Apply zoom** - scale for display
3. **Convert to image::DynamicImage** for ratatui-image
4. **Render via StatefulProtocol** which handles sixel encoding

```rust
fn render_canvas_to_sixel(&self, canvas: &SixelCanvas, area: Rect) -> Option<StatefulProtocol> {
    // Create image from canvas pixels
    let img = image::RgbaImage::from_raw(
        canvas.width,
        canvas.height,
        canvas.pixels.clone(),
    )?;

    // Apply zoom by scaling
    let zoomed = if canvas.zoom > 1 {
        image::imageops::resize(
            &img,
            canvas.width * canvas.zoom as u32,
            canvas.height * canvas.zoom as u32,
            image::imageops::FilterType::Nearest,  // Pixelated scaling
        )
    } else {
        img
    };

    // Convert to protocol
    let dyn_img = image::DynamicImage::ImageRgba8(zoomed);
    let mut picker = get_image_picker().lock().ok()?;
    Some(picker.new_resize_protocol(dyn_img))
}
```

### Tool Implementation

```rust
pub enum Tool {
    Pencil { size: u8 },
    Brush { size: u8, softness: u8 },
    Eraser { size: u8 },
    Line { start: Option<(u32, u32)> },
    ColorPicker,
    Text { font_size: u8 },
    Select { region: Option<Rect> },
}

impl Tool {
    /// Apply tool at position
    pub fn apply(&self, canvas: &mut SixelCanvas, x: u32, y: u32, color: (u8,u8,u8,u8)) {
        match self {
            Tool::Pencil { size } => {
                draw_circle(canvas, x, y, *size, color);
            }
            Tool::Brush { size, softness } => {
                draw_soft_circle(canvas, x, y, *size, *softness, color);
            }
            Tool::Eraser { size } => {
                draw_circle(canvas, x, y, *size, (0, 0, 0, 0));  // Transparent
            }
            // etc.
        }
    }
}
```

### Zoom and Pan

```rust
impl SixelCanvas {
    /// Convert terminal position to canvas pixel
    pub fn terminal_to_pixel(&self, term_x: u16, term_y: u16) -> (u32, u32) {
        // Assuming 8 pixels per char width, 6 pixels per char height for sixel
        let pixel_x = (term_x as u32 * 8) / self.zoom as u32 + self.scroll_x;
        let pixel_y = (term_y as u32 * 6) / self.zoom as u32 + self.scroll_y;
        (pixel_x.min(self.width - 1), pixel_y.min(self.height - 1))
    }

    /// Zoom in (max 16x)
    pub fn zoom_in(&mut self) {
        if self.zoom < 16 {
            self.zoom *= 2;
        }
    }

    /// Zoom out (min 1x)
    pub fn zoom_out(&mut self) {
        if self.zoom > 1 {
            self.zoom /= 2;
        }
    }

    /// Pan viewport
    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.scroll_x = (self.scroll_x as i32 + dx)
            .clamp(0, self.width as i32 - 1) as u32;
        self.scroll_y = (self.scroll_y as i32 + dy)
            .clamp(0, self.height as i32 - 1) as u32;
    }
}
```

## Color Palettes

### CGA Palette (4 colors)
```rust
const CGA_PALETTE: [(u8,u8,u8); 4] = [
    (0, 0, 0),       // Black
    (85, 255, 255),  // Cyan
    (255, 85, 255),  // Magenta
    (255, 255, 255), // White
];
```

### EGA Palette (16 colors)
```rust
const EGA_PALETTE: [(u8,u8,u8); 16] = [
    (0, 0, 0),       // 0: Black
    (0, 0, 170),     // 1: Blue
    (0, 170, 0),     // 2: Green
    (0, 170, 170),   // 3: Cyan
    (170, 0, 0),     // 4: Red
    (170, 0, 170),   // 5: Magenta
    (170, 85, 0),    // 6: Brown
    (170, 170, 170), // 7: Light Gray
    (85, 85, 85),    // 8: Dark Gray
    (85, 85, 255),   // 9: Light Blue
    (85, 255, 85),   // A: Light Green
    (85, 255, 255),  // B: Light Cyan
    (255, 85, 85),   // C: Light Red
    (255, 85, 255),  // D: Light Magenta
    (255, 255, 85),  // E: Yellow
    (255, 255, 255), // F: White
];
```

### VGA Palette (256 colors)
Use the standard VGA 256-color palette for retro games.

### Full RGB
For professional work, allow full 24-bit color picking.

## File I/O

### Supported Formats
- **PNG** - Preferred for pixel art (lossless, alpha)
- **GIF** - For palette-limited art (256 colors)
- **BMP** - Windows bitmap format

```rust
use image::{ImageFormat, RgbaImage};

pub fn load_image(path: &Path) -> Result<SixelCanvas, String> {
    let img = image::open(path)
        .map_err(|e| format!("Failed to load: {}", e))?
        .to_rgba8();

    Ok(SixelCanvas {
        width: img.width(),
        height: img.height(),
        pixels: img.into_raw(),
        ..Default::default()
    })
}

pub fn save_image(canvas: &SixelCanvas, path: &Path) -> Result<(), String> {
    let img = RgbaImage::from_raw(canvas.width, canvas.height, canvas.pixels.clone())
        .ok_or("Invalid canvas data")?;

    let format = match path.extension().and_then(|e| e.to_str()) {
        Some("png") => ImageFormat::Png,
        Some("gif") => ImageFormat::Gif,
        Some("bmp") => ImageFormat::Bmp,
        _ => ImageFormat::Png,
    };

    img.save_with_format(path, format)
        .map_err(|e| format!("Failed to save: {}", e))
}
```

## Game Graphics

### Splash Screen Assets

Generated splash screens live in `assets/splash/` (created via nano-banana-pro skill):

| File | Game | Theme |
|------|------|-------|
| `cosmos.png` | COSMOS | Space exploration, starfield, spacecraft |
| `rogue.png` | ROGUE | Dungeon crawler, skeleton, treasure |
| `trek.png` | TREK | Star Trek bridge, viewscreen |
| `dungeon.png` | DUNGEON | Fantasy RPG, dragon, castle |
| `biolab.png` | BIOLAB | Sci-fi horror, toxic lab |
| `gumshoe.png` | GUMSHOE | Noir detective, rainy city |
| `westworld.png` | WESTWORLD | Wild west, saloon, sunset |

**Generate more splash screens:**
```bash
SKILL_DIR="/path/to/nano-banana-pro/skills/generate"
uv run "${SKILL_DIR}/scripts/image.py" \
  --prompt "Retro DOS VGA pixel art game title screen for 'GAMENAME'. 1990s 16-bit style, [theme], blocky bitmap font title" \
  --output "assets/splash/gamename.png" \
  --aspect landscape --model flash
```

### Splash Screen Implementation

```rust
/// Embed splash image at compile time
const SPLASH_IMAGE: &[u8] = include_bytes!("../../../assets/splash/cosmos.png");

/// Load and display a splash screen
pub fn show_splash(frame: &mut Frame, image_data: &[u8], area: Rect) {
    if let Ok(img) = image::load_from_memory(image_data) {
        if let Ok(mut picker) = get_image_picker().lock() {
            let mut protocol = picker.new_resize_protocol(img);
            let widget = StatefulImage::new(None);
            frame.render_stateful_widget(widget, area, &mut protocol);
        }
    }
}
```

### Sprite Rendering

For games, render sprites to a buffer then composite:

```rust
pub struct Sprite {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,  // RGBA
}

impl Sprite {
    /// Draw sprite onto canvas with transparency
    pub fn draw_to(&self, canvas: &mut SixelCanvas, x: u32, y: u32) {
        for dy in 0..self.height {
            for dx in 0..self.width {
                let src_idx = ((dy * self.width + dx) * 4) as usize;
                let alpha = self.pixels[src_idx + 3];

                if alpha > 0 {
                    let color = (
                        self.pixels[src_idx],
                        self.pixels[src_idx + 1],
                        self.pixels[src_idx + 2],
                        alpha,
                    );
                    canvas.set_pixel(x + dx, y + dy, color);
                }
            }
        }
    }
}
```

## Sound Feedback

Q-PAINT should provide audio feedback for actions:

```rust
use rodio::{Sink, OutputStream};

pub enum SoundEffect {
    Draw,      // Soft click
    Undo,      // Swoosh backward
    Redo,      // Swoosh forward
    Save,      // Confirmation beep
    Error,     // Error buzz
    PickColor, // Plop sound
}

impl SoundEffect {
    pub fn play(&self) {
        // Use existing R-DOS sound infrastructure
        // See src/plugins/audio/ for pattern
    }
}
```

## Performance Tips

1. **Batch pixel operations** - Don't render after every pixel
2. **Use dirty rectangles** - Only re-render changed regions
3. **Cache protocol state** - Reuse StatefulProtocol when possible
4. **Limit undo history** - Canvas snapshots are large

## UI Layout for Q-PAINT

```
╔═══════════════════════ Q-PAINT ═══════════════════════════════════╗
║ File  Edit  View  Tools  Colors                             ?Help ║
╠═══════════════════════════════════════════════════════════════════╣
║ [Pencil][Brush][Eraser][Line][Text][Pick][Select]  Size:[3] +-    ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                              ┌───┐║
║                                                              │   │║
║                    [SIXEL CANVAS]                            │ P │║
║                                                              │ A │║
║                                                              │ L │║
║                                                              │ E │║
║                                                              │ T │║
║                                                              │ T │║
║                                                              │ E │║
║                                                              └───┘║
╠═══════════════════════════════════════════════════════════════════╣
║ Pos:(32,48) | Zoom:4x | 64x64 | FG:#FFFFFF BG:#000000 | Modified  ║
╚═══════════════════════════════════════════════════════════════════╝
 ^O:Open ^S:Save ^Z:Undo ^Y:Redo Z:Zoom Tab:Palette Arrows:Move Esc
```

## Key Bindings (Q-PAINT)

| Key | Action |
|-----|--------|
| Arrow keys | Move cursor |
| Space | Draw with current tool |
| Shift+Arrow | Draw while moving |
| P | Pencil tool |
| B | Brush tool |
| E | Eraser tool |
| L | Line tool |
| T | Text tool |
| I | Color picker (eyedropper) |
| S | Select tool |
| Z / Shift+Z | Zoom in/out |
| +/- | Brush size |
| Tab | Color palette |
| 0-9 | Quick palette colors |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+C | Copy selection |
| Ctrl+V | Paste |
| Ctrl+X | Cut selection |
| Ctrl+S | Save |
| Ctrl+O | Open |
| Ctrl+N | New |
| F1 or ? | Help |
| Esc | Exit |

## Testing Sixel Support

```rust
fn check_sixel_support() -> bool {
    if let Ok(picker) = Picker::from_query_stdio() {
        // Check if picker selected sixel or kitty protocol
        // (Both support pixel graphics)
        true
    } else {
        false
    }
}
```

## Vector Graphics

### Protocol Landscape

Terminal vector graphics protocols exist but have very limited modern support:

| Protocol | Era | Modern Support | Notes |
|----------|-----|----------------|-------|
| **ReGIS** | 1980s DEC | xterm only (recompile required) | Native vector, very efficient |
| **Tektronix 4010/4014** | 1970s | xterm, some BBS clients | Vector display emulation |
| **CTX/NAPLPS/RIPscrip** | 1980s-90s | Essentially none | BBS-era graphics |

**Practical Reality**: No modern terminal (Kitty, WezTerm, iTerm2, foot) supports native vector graphics. The only viable approach is **rasterization**.

### SVG Rendering Strategy

Render SVG to bitmap, then display via sixel:

```toml
[dependencies]
resvg = "0.44"           # SVG rendering
usvg = "0.44"            # SVG parsing
tiny-skia = "0.11"       # 2D rendering backend
ratatui-image = "3.0"    # Sixel display
```

```rust
use resvg::{tiny_skia, usvg};

/// Render SVG to pixels for sixel display
pub fn render_svg(svg_data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    // Parse SVG
    let tree = usvg::Tree::from_data(svg_data, &usvg::Options::default()).ok()?;

    // Create pixel buffer
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    // Calculate scale to fit
    let svg_size = tree.size();
    let scale_x = width as f32 / svg_size.width();
    let scale_y = height as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    // Render
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(pixmap.take())
}

/// Display SVG via sixel
pub fn display_svg(frame: &mut Frame, svg_data: &[u8], area: Rect) {
    let width = area.width as u32 * 8;   // Approximate pixel width
    let height = area.height as u32 * 16; // Approximate pixel height

    if let Some(pixels) = render_svg(svg_data, width, height) {
        let img = image::RgbaImage::from_raw(width, height, pixels)?;
        let dyn_img = image::DynamicImage::ImageRgba8(img);

        if let Ok(mut picker) = get_image_picker().lock() {
            let mut protocol = picker.new_resize_protocol(dyn_img);
            let widget = StatefulImage::new(None);
            frame.render_stateful_widget(widget, area, &mut protocol);
        }
    }
}
```

### Retro Wireframe Rendering

For that classic Star Wars/Battlezone vector look, render vectors to bitmap with phosphor effect:

```rust
/// Phosphor green colors (mimics CRT persistence)
const PHOSPHOR_GREEN: (u8, u8, u8) = (0, 255, 0);       // Bright line
const PHOSPHOR_DIM: (u8, u8, u8) = (0, 128, 0);         // Glow
const PHOSPHOR_DARK: (u8, u8, u8) = (0, 64, 0);         // Fade

/// Wireframe 3D model for retro rendering
pub struct WireframeModel {
    pub vertices: Vec<(f32, f32, f32)>,  // 3D points
    pub edges: Vec<(usize, usize)>,       // Vertex index pairs
}

/// Render wireframe to pixel buffer with phosphor effect
pub fn render_wireframe(
    model: &WireframeModel,
    width: u32,
    height: u32,
    rotation: (f32, f32, f32),
) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Project 3D to 2D
    let projected: Vec<(i32, i32)> = model.vertices.iter()
        .map(|v| project_vertex(*v, rotation, width, height))
        .collect();

    // Draw edges with anti-aliased phosphor glow
    for &(a, b) in &model.edges {
        let (x0, y0) = projected[a];
        let (x1, y1) = projected[b];

        // Draw glow (wider, dimmer)
        draw_line_aa(&mut pixels, width, height, x0, y0, x1, y1, PHOSPHOR_DIM, 3);
        // Draw core line (bright)
        draw_line_aa(&mut pixels, width, height, x0, y0, x1, y1, PHOSPHOR_GREEN, 1);
    }

    pixels
}

fn project_vertex(v: (f32, f32, f32), rot: (f32, f32, f32), w: u32, h: u32) -> (i32, i32) {
    // Simple perspective projection
    let (rx, ry, rz) = rot;
    let (x, y, z) = v;

    // Rotate
    let cos_x = rx.cos(); let sin_x = rx.sin();
    let cos_y = ry.cos(); let sin_y = ry.sin();
    let y1 = y * cos_x - z * sin_x;
    let z1 = y * sin_x + z * cos_x;
    let x1 = x * cos_y + z1 * sin_y;
    let z2 = -x * sin_y + z1 * cos_y;

    // Perspective divide
    let scale = 200.0 / (z2 + 400.0);
    let px = (x1 * scale + w as f32 / 2.0) as i32;
    let py = (y1 * scale + h as f32 / 2.0) as i32;

    (px, py)
}
```

### Common Vector File Formats

| Format | Crate | Use Case |
|--------|-------|----------|
| **SVG** | `resvg`, `usvg` | Modern vector graphics |
| **DXF** | `dxf-rs` | CAD drawings |
| **HPGL** | custom | Plotter files |
| **OBJ** (wireframe) | `obj-rs` | 3D wireframes |

### Viewer Plugin Pattern

For the file viewer to support SVG:

```rust
// In src/plugins/viewer/mod.rs

fn detect_mode(file_name: &str) -> ViewMode {
    match file_name.rsplit('.').next().map(|s| s.to_lowercase()).as_deref() {
        Some("svg") => ViewMode::Vector,
        Some("png" | "jpg" | "gif" | "bmp" | "webp") => ViewMode::Image,
        // ... other modes
    }
}

fn render_vector_view(frame: &mut Frame, area: Rect, state: &ViewerState, colors: &ThemeColors) {
    // Render SVG content as rasterized image
    if let Some(pixels) = render_svg(&state.content, area.width as u32 * 8, area.height as u32 * 16) {
        // Display via sixel...
    } else {
        // Show XML source as fallback
        render_normal_view(frame, area, state, colors);
    }
}
```

### Terminal Graphics Protocol Summary

From community research:

**Sixel Support (bitmap protocol)**:
- xterm, contour, foot, mintty, mlterm, RLogin, wezterm - Good support
- iTerm2, MacTerm - Works with quirks
- alacritty (graphics branch), VTE (wip/sixels), konsole - Experimental

**Kitty Protocol (bitmap)**:
- kitty, wezterm, konsole - Good support

**iTerm2 Protocol (bitmap)**:
- iTerm2, wezterm, mintty, konsole, hterm - Good support

**Vector (ReGIS/Tektronix)**:
- xterm only (requires `--enable-regis-graphics` compile flag)
- Not practical for general use

### CRT/Phosphor Visual Effects

For authentic retro aesthetics:

```rust
/// Apply CRT scanline effect
pub fn apply_scanlines(pixels: &mut [u8], width: u32, height: u32, intensity: f32) {
    for y in 0..height {
        // Darken every other line
        if y % 2 == 1 {
            let row_start = (y * width * 4) as usize;
            for x in 0..width {
                let idx = row_start + (x * 4) as usize;
                pixels[idx] = (pixels[idx] as f32 * intensity) as u8;
                pixels[idx + 1] = (pixels[idx + 1] as f32 * intensity) as u8;
                pixels[idx + 2] = (pixels[idx + 2] as f32 * intensity) as u8;
            }
        }
    }
}

/// Apply phosphor bloom/glow effect
pub fn apply_bloom(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    // Simple box blur for glow effect
    // ... implementation
}
```

## References

- [VT340 Sixel Graphics](https://vt100.net/docs/vt3xx-gp/chapter14.html)
- [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [ReGIS Protocol](https://en.wikipedia.org/wiki/ReGIS) - DEC vector graphics
- [VT340 Test Suite](https://github.com/hackerb9/vt340test) - Terminal compatibility tests
- [ratatui-image crate](https://github.com/benjajaja/ratatui-image)
- [resvg crate](https://github.com/RazrFalcon/resvg) - SVG rendering
- [Aseprite](https://www.aseprite.org/) - Professional pixel art tool (inspiration)
- [Pixcil](https://github.com/sile/pixcil) - Rust pixel editor
- [timg](https://github.com/hzeller/timg) - Terminal image viewer
