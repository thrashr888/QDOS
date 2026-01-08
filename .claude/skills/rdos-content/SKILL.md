---
name: rdos-content
description: Help text, error messages, and strings for R-DOS. Use when writing user-facing text, help content, error messages, or modal labels. Reference spec/help.txt and spec/strings/ for authentic Q-DOS II phrasing.
---

# R-DOS Content Guide

R-DOS recreates Q-DOS II (1991) authentic text and messaging style.

## Reference Files

- `spec/help.txt` - Original Q-DOS II help text
- `spec/strings/` - Organized strings by feature
- `spec/qdos-strings.txt` - Complete string dump

## Writing Style

### Tone
- Direct, concise DOS-era language
- No emojis or modern slang
- Technical but accessible

### Capitalization
- Commands: ALL CAPS (COPY, ERASE, RENAME)
- Titles: Title Case
- Body text: Sentence case

### Error Messages

Q-DOS II style:
```
*** ERROR ***

Unable to open file: permission denied

Press any key to continue
```

In code:
```rust
self.state.error = Some("Unable to open file: permission denied".to_string());
self.state.view = MyView::Error;
```

## Help Content Pattern

```rust
fn help_content(&self) -> Vec<String> {
    vec![
        "My Plugin".to_string(),
        "".to_string(),
        "Brief description of what this does.".to_string(),
        "".to_string(),
        "Features:".to_string(),
        "  Feature 1 - Description".to_string(),
        "  Feature 2 - Description".to_string(),
        "".to_string(),
        "Keyboard shortcuts:".to_string(),
        "  Enter  - Confirm action".to_string(),
        "  Esc    - Cancel/Close".to_string(),
    ]
}
```

## Key Binding Labels

Standard format in help bars:
```rust
vec![
    ("Enter", "confirm"),
    ("Esc", "cancel"),
    ("↑↓", "navigate"),
    ("Space", "toggle"),
    ("F1", "help"),
]
```

## Modal Titles

- Centered, surrounded by spaces: `" TITLE "`
- Action-oriented: `" CONFIRM DELETE "`, `" SELECT FILE "`
- Status included when relevant: `" Playing [▶] "`

## Progress Messages

```
Copying FILE.TXT ====> /destination/path
Processing item 3 of 10 [████████░░░░░░░░] 30%
```

## Common Phrases

From original Q-DOS II:
- "Press any key to continue"
- "Press ESC to abort"
- "Press RETURN to confirm"
- "Operation complete"
- "No files found"
- "Invalid selection"
