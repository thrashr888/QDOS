# Ghostty and modern terminal integration

## Kitty image previews

Open a PNG, JPEG, GIF, or SVG in the file viewer (`rdos view image.png`).
R-DOS selects Kitty graphics for a direct interactive Ghostty or Kitty terminal
(`TERM_PROGRAM=ghostty`/`kitty` or `TERM=xterm-ghostty`/`xterm-kitty`). It does not
start a stdin capability probe: ratatui-image 3's probe can leave a reader running
after timeout, competing with keyboard input. Protocol selection happens once
at startup, never during rendering.

Other direct terminals retain the library's environment-based iTerm2 selection
where available and otherwise use half-blocks. Sixel terminals that previously
relied on the query now need an explicit override. To select a known capability:

```sh
RDOS_IMAGE_PROTOCOL=sixel rdos    # Known sixel terminal
RDOS_IMAGE_PROTOCOL=kitty rdos    # Known Kitty-protocol terminal
RDOS_IMAGE_PROTOCOL=iterm2 rdos   # Known iTerm2-protocol terminal
RDOS_IMAGE_PROTOCOL=halfblocks rdos # Always use character-cell fallback
```

Unknown values mean auto. Redirected stdin/stdout always selects half-blocks,
even with a graphics override. tmux/screen defaults to half-blocks instead of
trusting inherited outer-terminal variables. An explicit graphics override under
tmux opts into ratatui-image's existing routing, which may enable that pane's
`allow-passthrough` option. Without an override, R-DOS does not modify it.
Cell pixel dimensions come from the OS window-size report when available;
otherwise R-DOS uses an approximate 8x16 fallback, not measured font metrics.

The viewer retains one image protocol per open preview. Repeated UI ticks reuse
the transmitted Kitty image instead of decoding and retransmitting it with a new
ID every frame. Resizing re-encodes the same image ID for the new dimensions.
Changing content or view mode, reopening, and closing invalidate the cache. SVG
rasterization is cached as well. Failed decodes are cached until content changes.
On a direct terminal, invalidation deletes only the viewer's own Kitty image ID.
With an explicit graphics override, multiplexers retain the library's routing; explicit image-data deletion
is skipped there because ratatui-image 3 does not expose its routing state.
Virtual placements disappear when their placeholder text is overwritten, but
image data may remain in the multiplexer terminal until eviction.

Ghostty 1.4 completes Kitty animation and relative placement support; static
Kitty graphics existed before 1.4. This update improves static preview reuse,
resize behavior and resource lifetime. It does **not** implement animated GIF
playback, Kitty animation commands, or relative image placements.

## Native progress

Batch copy, move and erase operations report file-count progress to Ghostty's
native split progress bar using ConEmu OSC 9;4. This supplements the existing
in-app progress dialog; it is not a byte-level copy progress meter.

- Progress starts at zero before the first file and updates after each file.
- A background reporter repeats the current state once per second, including
  while a large synchronous file operation blocks the UI. Ghostty otherwise
  expires an unchanged indicator after approximately 15 seconds.
- A failed file changes the state to error while the remaining batch continues.
  The existing completion dialog reports the result; the native bar clears.
- Completion, Escape cancellation, leaving the progress dialog, normal exit,
  and returned errors clear the indicator. Unwinding also drops the reporter.
  Forced termination and release-mode panic abort cannot run cleanup; Ghostty's
  timeout is the fallback in those cases.
- Output is serialized with frame rendering so heartbeats cannot split a Kitty
  image transfer. No clipboard or notification operations are performed.

Auto detection is conservative: stdout must be a terminal and `TERM_PROGRAM`
must be `ghostty` or `TERM` must be `xterm-ghostty`. tmux/screen sessions are off
by default. Other terminals keep the in-app dialog without extra OSC output.

Override when needed:

```sh
RDOS_TERMINAL_PROGRESS=off rdos  # Disable native progress
RDOS_TERMINAL_PROGRESS=on rdos   # Opt in for another known OSC 9;4 terminal
```

`on` never writes protocol sequences to redirected stdout. Unknown override
values behave like auto. Ghostty's own `progress-style` setting must allow the
indicator. Ghostty supported OSC 9;4 before 1.4; it is not a new 1.4 protocol.

## Acceptance checks on a real terminal

Automated tests cover cache invalidation, actual Kitty transmit/reuse/resize
buffers, progress encoding, conservative detection, heartbeat and clear.
They do not replace these native terminal checks:

| Scenario | Expected result |
| --- | --- |
| Ghostty 1.4: open PNG/JPEG/SVG, idle several seconds | Stable image, no repeated upload/flicker |
| Resize preview, switch files/modes, close and reopen | Correct image and dimensions; no stale placements |
| Invalid image then valid image | Error/fallback then recovered preview |
| Unknown/non-graphics terminal | Half-block fallback, no forced Kitty or stdin query |
| Sixel terminal with `RDOS_IMAGE_PROTOCOL=sixel` | Sixel preview |
| Batch copy/move/erase lasting over 15 seconds | Native progress remains visible; percentage is file count |
| Escape between files, finish batch, operation errors, quit | Native indicator clears; dialog reports outcomes |
| Native progress disabled; tmux/screen default | In-app progress only |
| `on` on a known compatible terminal | Native progress without interleaved image data |

Cancellation remains between files, as before; a blocking file copy is not
interrupted mid-file. Test copy/move/erase only on disposable fixtures.

## Protocol references

- [Ghostty 1.4 release notes](https://ghostty.org/docs/install/release-notes/1-4-0)
- [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [Ghostty ConEmu OSC 9;4 states and keep-alive](https://ghostty.org/docs/vt/osc/conemu)
- [Ghostty progress configuration](https://ghostty.org/docs/config/reference#progress-style)
