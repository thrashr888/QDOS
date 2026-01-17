# Inline Video Playback for QDOS Video Plugin

## Overview
Add inline video playback to the video plugin using ffmpeg-sidecar for frame extraction, ratatui-image for terminal graphics rendering, and ASCII art fallback for unsupported terminals.

## User Requirements
- **Fallback**: ASCII art rendering when terminal doesn't support Kitty/Sixel/iTerm2
- **FFmpeg**: Detect and suggest install hint if missing, fall back to external player
- **Frame rate**: Target 10 fps

## Dependencies
```toml
# Add to Cargo.toml
ffmpeg-sidecar = "2.4"
```

## New Files

| File | Purpose |
|------|---------|
| `src/plugins/video/player.rs` | Background thread for ffmpeg frame extraction |
| `src/plugins/video/ascii.rs` | ASCII art rendering fallback |
| `src/plugins/video/ffmpeg.rs` | FFmpeg detection and install hints |

## Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` | Add ffmpeg-sidecar dependency |
| `src/plugins/video/state.rs` | Add PlaybackMode, PlayState, InlinePlaybackState, new VideoView variants |
| `src/plugins/video/mod.rs` | Add inline player handle, key handlers, tick() |
| `src/plugins/video/modal.rs` | Add draw_inline_player(), draw_ffmpeg_missing(), update menu |

## State Changes (state.rs)

```rust
pub enum PlaybackMode {
    Inline,    // ffmpeg + terminal graphics
    External,  // mpv, VLC, IINA (default)
}

pub enum VideoView {
    Menu,
    Playing,        // External player
    InlinePlayer,   // NEW: Inline playback
    FfmpegMissing,  // NEW: FFmpeg not found
    Error,
}

pub struct InlinePlaybackState {
    pub ffmpeg_available: bool,
    pub play_state: PlayState,
    pub current_frame: u64,
    pub position: f32,
    pub duration: f32,
    pub frame_data: Option<Vec<u8>>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub target_fps: u8,          // 10
    pub graphics_supported: bool,
}

pub enum PlayState {
    Stopped,
    Playing,
    Paused,
}
```

## Key Bindings

### Menu View
| Key | Action |
|-----|--------|
| `M` | Toggle Inline/External mode |
| `Enter` | Play with selected mode |

### Inline Player View
| Key | Action |
|-----|--------|
| `Space` | Play/pause |
| `S` | Stop |
| `Left/Right` | Seek 5s |
| `[` / `]` | Prev/next file |
| `M` | Switch to external mode |
| `Esc` | Close |

### FFmpeg Missing View
| Key | Action |
|-----|--------|
| `E` | Use external player |
| `Esc` | Close |

## Implementation Phases

### Phase 1: Foundation
1. Add ffmpeg-sidecar to Cargo.toml
2. Create `ffmpeg.rs` with `is_ffmpeg_available()`, `get_install_hint()`
3. Add new state types to state.rs
4. Update VideoState struct

### Phase 2: Background Player
1. Create `player.rs` following audio plugin pattern
2. Implement VideoPlayerHandle with command channel
3. Implement frame extraction using ffmpeg-sidecar:
   ```rust
   FfmpegCommand::new()
       .input(path)
       .args(["-vf", "fps=10"])
       .args(["-f", "rawvideo", "-pix_fmt", "rgb24"])
       .pipe_stdout()
       .spawn()
   ```
4. Add Arc<Mutex<InlinePlaybackState>> for thread sync

### Phase 3: ASCII Fallback
1. Create `ascii.rs`
2. Implement frame_to_ascii() using luminance calculation
3. Character set: `[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@']`

### Phase 4: UI Integration
1. Add draw_inline_player() - full-screen video with controls
2. Add draw_ffmpeg_missing() - install hint dialog
3. Update menu to show mode toggle
4. Use ratatui-image picker pattern from viewer plugin for graphics

### Phase 5: Key Handling
1. Add VideoView::InlinePlayer handlers
2. Add VideoView::FfmpegMissing handlers
3. Add mode toggle to menu
4. Implement seek, play/pause, stop, prev/next

### Phase 6: Polish
1. Implement tick() for frame sync (10 ticks/sec = 10 fps)
2. Add cleanup in Drop
3. Test various video formats
4. Test ASCII fallback

## Architecture

```
VideoPlugin
├── state: VideoState
│   ├── playback_mode: PlaybackMode
│   └── inline_state: InlinePlaybackState
├── inline_player: Option<VideoPlayerHandle>
└── player_state: Option<Arc<Mutex<InlinePlaybackState>>>

VideoPlayerHandle (player.rs)
├── command_tx: Sender<VideoCommand>
└── Background thread:
    ├── ffmpeg-sidecar process
    ├── Frame extraction loop (10 fps)
    └── State updates via Arc<Mutex>
```

## Error Handling
1. FFmpeg not installed → Show FfmpegMissing view with install hint
2. FFmpeg fails → Error view, allow external fallback
3. Graphics not supported → Auto-use ASCII fallback
4. Frame extraction fails → Continue or fallback

## Reference Files
- `src/plugins/audio/player.rs` - Background thread pattern
- `src/plugins/viewer/mod.rs` - ratatui-image integration (IMAGE_PICKER)
- `src/plugins/audio/modal.rs` - Playback controls UI pattern
