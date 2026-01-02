# R-DOS Specification

A detailed specification for recreating Q-DOS II as a modern Rust TUI application.

## Reference Materials

- `/images/` - Screenshots from original Q-DOS II
- `/spec/ui.md` - ASCII layout of main screen
- `/spec/help.txt` - Extracted help text content
- `/spec/qdos-strings.txt` - Raw extracted strings from disk image (2,815 lines)
- `/spec/strings/` - Organized string files by feature:
  - `attribute.txt` - File attribute commands and options
  - `config.txt` - QDSTART & QDCOLOR configuration
  - `copy.txt` - Copy dialogs, messages, errors
  - `directory.txt` - Directory navigation, map, make/remove
  - `dos.txt` - DOS/shell command execution
  - `edit.txt` - Q-EDIT commands and navigation
  - `erase.txt` - Erase confirmations and warnings
  - `errors.txt` - All error messages by category
  - `find.txt` - File search dialog and options
  - `menu.txt` - Main menu, function keys, UI elements
  - `move.txt` - Move dialogs and errors
  - `print.txt` - Print options and printer errors
  - `rename-space.txt` - Rename and disk space commands
  - `tag.txt` - Tag/untag system
  - `view.txt` - File viewer, hex mode, navigation
- [YouTube Demo](https://www.youtube.com/watch?v=82j6NpSDTWQ)

---

## Design Principles

### Destructive Action Confirmation
All destructive actions (delete, erase, overwrite, etc.) MUST require explicit user confirmation before execution. This includes:
- Directory deletion
- File erasure
- Overwriting existing files during copy/move
- Any operation that cannot be undone

The original Q-DOS II had a configurable "Confirm Delete" option in QDSTART. R-DOS defaults to requiring confirmation, with the option to disable it via settings (when implemented).

---

## 1. Screen Layout (80x25)

### 1.1 Main Screen Structure

```
Row 1:   Menu Bar (white text, selected = yellow on red)
Row 2:   Menu Description (green text)
Row 3:   Separator (═══════════════════════════════════════════════════════════════════════════════)
Row 4:   PATH Bar (PATH >> [path in yellow on red extending to right edge])
Row 5:   Top border + column headers separator
Row 6:   Stats header | File table header row (blue text)
Row 7+:  Stats panel  | File listing (scrollable)
...
Row 22:  Keybindings section (single line borders)
Row 23:  Keybindings continued
Row 24:  Copyright line 1
Row 25:  Copyright line 2
```

### 1.2 Left Panel (30 chars wide)

```
 Count          Total Size
╔════╗        ╔═══════════╗
║  71║ Files  ║    631,849║
╠════╣        ╚═══════════╝
║  22║ Directories
╠════╣        ╔═══════════╗
║   2║ Tagged ║      1,249║
╚════╝        ╚═══════════╝
─────────────────────────────
 F1- Help       F2- Status
 F3- Chg Drive  F4- Prev Dir
 F5- Chg Dir    F6- DOS Cmd
 F7- Srch Spec  F8- Sort
 F9- Edit      F10- Quit
   SPACE BAR- Tag file
   ESC- Abort Command
─────────────────────────────
 Q-DOS II — Version 2.00
   Copyright (c) 1986
GAZELLE SYSTEMS - Provo, Utah
```

### 1.3 Right Panel (File Table)

```
╦══════════════╦════════════╦══════════╦═════════╗
║  File Name   ║   Size     ║   Date   ║   Time  ║
╬══════════════╬════════════╬══════════╬═════════╣
║ ADJDOS  .COM        3,422    7-29-87    12:00p ║
║ ANSI    .SYS ║      2,256 ║  1-29-87 ║   3:00p ║
...
╩══════════════╩════════════╩══════════╩═════════╝
```

**File Name Column Format:**
- 8 chars for name (uppercase, left-aligned)
- 1 char space or dot
- 3 chars for extension (uppercase)
- Directories show name only, no extension
- `..` for parent directory

**Size Column Format:**
- Right-aligned with comma separators
- `<DIR>` for directories
- Blank for `..` entry

**Date Column Format:**
- `M-DD-YY` with space-padded month/day (e.g., ` 1- 2-87`)

**Time Column Format:**
- `HH:MMp` with `a` or `p` suffix (e.g., `12:00p`, ` 3:47a`)

---

## 2. Color Scheme

### 2.1 DOS 16-Color Palette (RGB Values)

| Color   | Use                          | RGB Value        |
|---------|------------------------------|------------------|
| Black   | Background                   | `(0, 0, 0)`      |
| White   | Primary text, borders        | `(255, 255, 255)`|
| Blue    | Headers, menu items, copyright| `(102, 183, 179)`|
| Green   | Descriptions, help text      | `(103, 204, 77)` |
| Red     | Selection background         | `(157, 31, 20)`  |
| Yellow  | Selected text, tagged files  | `(232, 218, 89)` |

### 2.2 Color Application

| Element                    | Foreground | Background |
|----------------------------|------------|------------|
| Menu bar (unselected)      | White      | Black      |
| Menu bar (selected)        | Yellow     | Red        |
| Menu description           | Green      | Black      |
| PATH label                 | White      | Black      |
| PATH value                 | Yellow     | Red (full width)|
| Table headers              | Blue       | Black      |
| Table borders              | White      | Black      |
| File entries (normal)      | White      | Black      |
| File entries (directory)   | Blue       | Black      |
| File entries (selected)    | Yellow     | Red        |
| File entries (tagged)      | Yellow     | Black      |
| Stats panel text           | White      | Black      |
| Stats panel borders        | White      | Black      |
| Keybindings                | White      | Black      |
| Keybindings border         | White (single line) | Black |
| Copyright                  | Blue       | Black      |

---

## 3. Menu System

### 3.1 Main Menu Items

| Item      | Key | Description |
|-----------|-----|-------------|
| Directory | D   | Change current directory, make or remove directory, see directory tree |
| Tag       | T   | Tag groups of files, or clear all tags -- SPACE BAR tags highlighted file |
| View      | V   | View the contents of any file on the screen (in "ASCII" or "HEX") |
| Copy      | C   | Copy one or several files to another disk or directory |
| Move      | M   | Move one or several files from this directory to another directory |
| Find      | F   | Search all directories on the disk to find specified file(s) |
| Erase     | E   | Erase one or several files from this directory |
| Rename    | R   | Rename one or several files in this directory |
| Space     | S   | Show the total, used, and free space on any disk |
| Attribute | A   | Change/view file attributes |
| Print     | P   | Print one or several files on the printer |

### 3.2 Submenus

**Directory Submenu:**
- Change Directory
- Make Directory
- Remove Directory
- Directory Map (tree view)
- Scan Drive

**Tag Submenu:**
- Tag All
- Untag All
- Retag (re-select previous tags)
- Select (by pattern)

**Copy/Move Submenu:**
- Highlighted (single file)
- Tagged (batch operation)

**Erase Submenu:**
- Highlighted
- Tagged

**Rename Submenu:**
- Highlighted
- Tagged

**Attribute Submenu:**
- Highlighted
- Tagged
- Display (change display mode)

**View Submenu:**
- ASCII
- HEX

---

## 4. Function Keys

| Key | Action              | Description |
|-----|---------------------|-------------|
| F1  | Help                | Show help screen |
| F2  | Status              | Show system status screen |
| F3  | Change Drive        | Prompt for drive letter |
| F4  | Previous Directory  | Go to parent directory |
| F5  | Change Directory    | Prompt for path |
| F6  | Shell Command       | Execute shell command (streams output) |
| F7  | Search Spec         | Set file search specification (e.g., `*.EXE`) |
| F8  | Sort                | Cycle sort mode |
| F9  | Edit                | Open file in editor |
| F10 | Quit                | Exit Q-DOS II |

---

## 5. Screens and Dialogs

### 5.1 Quit Dialog (Overlay)

```
╔═══════════════════════════════════════╗
║       F10 - Quit Q-DOS II             ║
║                                       ║
║  Press F10 again to quit, or RETURN   ║
║              for options              ║
║                                       ║
║   Press ESC to return to Q-DOS II     ║
╚═══════════════════════════════════════╝
```

### 5.2 Space Dialog (Two-step)

**Step 1 - Drive Selection:**
```
╔═══════════════════════════════════════╗
║           Space On Disk               ║
║                                       ║
║     Check space on drive:             ║
║                                       ║
║   Enter drive letter, or press ESC    ║
╚═══════════════════════════════════════╝
```

**Step 2 - Results:**
```
╔═══════════════════════════════════════╗
║         Space On Disk A               ║
║                                       ║
║   Total space:         362,496        ║
║                                       ║
║   Total used:          248,832        ║
║                                       ║
║   Total available:     113,664        ║
║                                       ║
║     Press any key to continue         ║
╚═══════════════════════════════════════╝
```

### 5.3 Rename Screen (Full Screen)

Takes over entire screen:
```
 Renaming From: D:\
    (Type the new name for the file, or press ESC to stop)
═══════════════════════════════════════════════════════════════════════════════
Rename the file     to [______________] (input field with red background)

 Renaming this file  ===>




                    Press ESC to stop Renaming
```

### 5.4 Directory Map Screen (Full Screen)

```
                        Q-DOS II Directory Map
Path >> [current path with red background]
═══════════════════════════════════════════════════════════════════════════════
╔═══════════════════════════════════════════════════════════════════════════╗
║                            Scan Drive                                      ║
║                                                                            ║
║         Enter letter of disk drive: _                                      ║
║                                                                            ║
╚═══════════════════════════════════════════════════════════════════════════╝




(M)ake directory  (S)can disk  (L)og on to disk  (R)emove directory
    Use arrow keys to move cursor, RETURN to select, or ESC to quit
```

### 5.5 Configuration Screen

```
                    Q-DOS II Configuration Subprogram

                              NORM  DIR   HID   SYS   R/O   ARC
Search Attributes ========>   Yes   Yes   No    No    No    No

Search Specification =====>   *.*

Sort Method =============>    Name

Sort Direction ==========>    Ascending

Q-DOS II Log File ========>   Yes

First Logged Drive =======>   C:

On-Line Editor Path =====>    C:\QDOS\QED.EXE

On-Line Help File Path ===>   C:\QDOS\QD2.HLP

───────────────────────────────────────────────────────────────────────────────
To change the search attribute settings, press the space bar.
For information about the search
```

### 5.6 Help Screen (F1)

Full-screen help viewer that displays content from `/spec/help.txt` (or embedded).

**Index Screen:**
```
                Q-DOS II  --  Index to Online Help

    I - Introduction to Q-DOS II    F1 - Help
    A - Attribute                   F2 - Status Screen
    C - Copy                        F3 - Change Drive
    D - Directory                   F4 - Previous Directory
    E - Erase                       F5 - Change Directory
    F - Find                        F6 - DOS Command
    M - Move                        F7 - Set Search Specifications
    P - Print                       F8 - Sort Files
    R - Rename                      F9 - Edit
    S - Space                      F10 - Quit
    T - Tag
    V - View
    1 - APPENDIX A -- Valid File Names in DOS
    2 - APPENDIX B -- Organizing a Hard Disk
    3 - APPENDIX C -- Error Messages

    PgUp - Previous page   PgDn - Next page   (I)ndex   ESC - Quit
```

**Navigation:**
- Press letter/number to jump to topic
- PgUp/PgDn to scroll
- "I" to return to index
- "G" for general/intro help
- ESC to exit help

**Help Topics:**
- Introduction to Q-DOS II
- All menu commands (Attribute, Copy, Directory, Erase, Find, Move, Print, Rename, Space, Tag, View)
- All function keys (F1-F10)
- Appendices (Valid File Names, Organizing Disk, Error Messages)

**Implementation:**
- Help text stored in `help.txt` or embedded as const string
- Built-in file viewer with scrolling
- Context-sensitive: F1 jumps to relevant section based on current command
- See `/spec/help.txt` for full help content

### 5.7 Status Screen

```
         System Status Screen
         (press any key to return to main screen)

[System information display]
- Memory usage
- Disk information
- DOS version
- etc.
```

### 5.8 Shell Command Screen (F6)

Full-screen command runner for executing shell commands:

```
                         R-DOS Shell Command
═══════════════════════════════════════════════════════════════════════════════
 Working Directory: /home/user/documents

 Enter command: ls -la_

═══════════════════════════════════════════════════════════════════════════════
 Output:







═══════════════════════════════════════════════════════════════════════════════
                    Press ESC to return to R-DOS
```

**After command execution (streaming output):**
```
                         R-DOS Shell Command
═══════════════════════════════════════════════════════════════════════════════
 Working Directory: /home/user/documents

 Command: ls -la

═══════════════════════════════════════════════════════════════════════════════
 Output:
 total 128
 drwxr-xr-x  12 user  staff   384 Jan  2 10:30 .
 drwxr-xr-x   5 user  staff   160 Jan  1 09:00 ..
 -rw-r--r--   1 user  staff  1234 Jan  2 10:15 document.txt
 -rw-r--r--   1 user  staff  5678 Jan  2 09:45 report.pdf
 drwxr-xr-x   3 user  staff    96 Jan  1 14:20 projects

 [Exit code: 0]
═══════════════════════════════════════════════════════════════════════════════
 Press ENTER for new command, ESC to return to R-DOS
```

**Features:**
- Executes commands in current working directory
- Streams stdout/stderr in real-time
- Shows exit code on completion
- Scrollable output for long results
- Command history (up/down arrows)
- Tab completion for commands and paths
- Supports pipes and redirects (`ls | grep foo`)
- Environment variables from parent shell

**Implementation:**
```rust
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

fn run_shell_command(cmd: &str, cwd: &Path) -> Result<()> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let mut child = Command::new(&shell)
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Stream output line by line
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        // Render line to screen
        render_output_line(&line?)?;
    }

    let status = child.wait()?;
    render_exit_code(status.code());
    Ok(())
}
```

**Common use cases:**
- `ls -la` - Detailed file listing
- `git status` - Check repository status
- `grep -r "pattern" .` - Search in files
- `cat file.txt` - View file contents
- `make build` - Run build commands
- `npm install` - Package management
- `docker ps` - Container status

---

## 6. File Operations

### 6.1 Tagging

- **Space Bar**: Toggle tag on highlighted file
- Tagged files shown in yellow
- Tag count displayed in stats panel
- Total size of tagged files shown

### 6.2 Copy Operation

1. Tag files to copy (or use highlighted for single)
2. Select Copy menu
3. Choose "Highlighted" or "Tagged"
4. Enter destination path
5. Show progress: `Copying [filename] ===> [destination]`
6. Handle conflicts (overwrite prompt)

### 6.3 Move Operation

Same as copy, but removes source after successful copy.

### 6.4 Erase Operation

1. Tag files to erase
2. Select Erase menu
3. Confirm deletion
4. Show progress: `Erasing [filename]`
5. Refresh file list

### 6.5 Rename Operation

**Single file:**
1. Highlight file
2. Select Rename > Highlighted
3. Full screen rename interface
4. Enter new name
5. Press Enter to confirm, ESC to cancel

**Batch rename:**
1. Tag files
2. Select Rename > Tagged
3. Rename each file sequentially
4. Can press ESC to stop at any point

---

## 7. Sorting

### 7.1 Sort Methods

| Method    | Description |
|-----------|-------------|
| Name      | Alphabetical by filename |
| Extension | Alphabetical by extension |
| Size      | By file size |
| Date      | By modification date |
| Unsorted  | DOS order (as stored) |

### 7.2 Sort Direction

- Ascending (A-Z, smallest first, oldest first)
- Descending (Z-A, largest first, newest first)

### 7.3 Sort Behavior

- Directories always listed first (after `..`)
- `..` always at top
- F8 cycles through sort modes

---

## 8. Search Specification

Filter files displayed using DOS wildcards:
- `*.*` - All files (default)
- `*.EXE` - Only .EXE files
- `A*.*` - Files starting with A
- `??CONFIG.*` - Files with CONFIG as chars 3-8

---

## 9. File Attributes

DOS file attributes that can be viewed/modified:

| Attr | Name      | Description |
|------|-----------|-------------|
| NORM | Normal    | Regular file |
| DIR  | Directory | Directory entry |
| HID  | Hidden    | Hidden from normal DIR |
| SYS  | System    | System file |
| R/O  | Read-Only | Cannot be modified |
| ARC  | Archive   | Needs backup |

---

## 10. Keyboard Navigation

### 10.1 File List Navigation

| Key       | Action |
|-----------|--------|
| ↑ / k     | Move up one file |
| ↓ / j     | Move down one file |
| PgUp      | Move up one page |
| PgDn      | Move down one page |
| Home      | Go to first file |
| End       | Go to last file |
| Enter     | Enter directory / Execute action |
| Space     | Tag/untag file |

### 10.2 Menu Navigation

| Key       | Action |
|-----------|--------|
| ← / h     | Previous menu item |
| → / l     | Next menu item |
| Enter     | Execute menu item |
| Letter    | Jump to menu item (D, T, V, C, M, F, E, R, S, A, P) |

### 10.3 Dialog Navigation

| Key       | Action |
|-----------|--------|
| Tab       | Cycle through options / Complete path |
| Enter     | Confirm |
| ESC       | Cancel / Close |
| Y / N     | Yes / No responses |

---

## 11. Implementation Status

### 11.1 Completed

- [x] Main screen layout
- [x] File listing with scrolling
- [x] Stats panel (file count, dir count, tagged count, sizes)
- [x] Navigation (arrow keys, PgUp/PgDn, Home/End)
- [x] Menu bar with selection
- [x] File tagging (Space bar)
- [x] Sort modes (F8 cycling)
- [x] Copy operation (tagged files)
- [x] Move operation (tagged files)
- [x] Erase operation (with confirmation)
- [x] Rename operation (single file)
- [x] Space dialog (disk space)
- [x] Help dialog (basic)
- [x] Status dialog (system info)
- [x] Quit confirmation
- [x] Path input with tab completion
- [x] Color scheme matching original
- [x] Box drawing characters
- [x] Responsive file name column
- [x] CLI argument for initial path

### 11.2 Planned

**Core Features:**
- [ ] Directory Map (tree view)
- [ ] Make/Remove directory
- [ ] View file contents (ASCII/HEX)
- [ ] Find files across directories
- [ ] Batch rename (tagged files)
- [ ] File attribute viewing/editing
- [ ] Print functionality
- [ ] Search specification filtering
- [ ] F3 Change Drive dialog
- [ ] F6 Shell Command runner (streaming output)
- [ ] F9 External editor integration
- [ ] Multi-page help system
- [ ] Full-screen rename interface
- [ ] Progress indicators for operations
- [ ] Error message dialogs (matching original text)

**R-DOS Enhancements:**
- [ ] Mouse support (click to select, double-click to open)
- [ ] Embedded QDCOLOR color theme changer
- [ ] Embedded QDSTART configuration subprogram
- [ ] Global config file persistence (~/.config/rdos/config.toml)
- [ ] Additional CLI options (--theme, --sort, --order)
- [ ] Q-EDIT built-in text editor (F9)
  - [ ] Normal text editing mode
  - [ ] Hex mode for binary files
  - [ ] Find/Replace with regex support
  - [ ] Block operations (buffer, copy, delete)
  - [ ] Markers (A-D) for navigation
  - [ ] Auto-indent and configurable tabs

### 11.3 Differences from Original

| Feature | Original Q-DOS II | R-DOS |
|---------|-------------------|-------|
| Drive letters | A:, B:, C: | Unix paths |
| DOS commands | COMMAND.COM | Shell commands |
| File attributes | DOS attrs | Unix permissions (rwx) |
| Printer | LPT1 | Not applicable |
| Log files | QD2N.LOG | Not implemented |
| Mouse support | None | Full mouse support |
| Config file | Binary in EXE | TOML in ~/.config |
| Color themes | QDCOLOR.COM separate | Embedded, multiple themes |
| Terminal size | Fixed 80x25 | Responsive (min 80x25) |
| CLI arguments | None | Path, theme, sort options |

---

## 12. Technical Notes

### 12.1 Terminal Requirements

- Minimum size: 80x25 characters
- Unicode support for box drawing characters
- 256-color or true-color support recommended

### 12.2 Box Drawing Characters

```
Single line: ─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼
Double line: ═ ║ ╔ ╗ ╚ ╝ ╠ ╣ ╦ ╩ ╬
Mixed:       ╒ ╓ ╘ ╙ ╞ ╟ ╤ ╥ ╧ ╨ ╪ ╫
```

### 12.3 Character Encoding

- File names displayed in uppercase (DOS convention)
- Extension separated by `.` or space
- 8.3 filename format for display compatibility

---

## Appendix A: Original Error Messages

From extracted strings:

```
*** ERROR: Invalid drive.
*** ERROR: Attempt to install Q-DOS II onto a floppy drive.
*** ERROR: Insufficient disk space.
*** ERROR: Source path same as destination path.
*** ERROR: Unable to create destination path.
*** ERROR: Unable to open source file
*** ERROR: Unable to create destination file
*** ERROR: Unable to execute QDCOLOR.COM
*** ERROR: Unable to open file QD2.EXE
```

---

## Appendix B: Original UI Text

From extracted strings:

```
Q-DOS II (C) Copyright 1988 -- All rights reserved!
Gazelle Systems
42 No. University Ave.
Suite 10
Provo, Utah 84601
1-800-233-0383 (1-801-377-1288 in Utah)

Press any key to continue...
Press PgDn for next page
Press ESC to return to Q-DOS II
```

---

## 13. R-DOS Enhancements

These features extend beyond the original Q-DOS II to provide a modern experience.

### 13.1 Mouse Support

Enable mouse interaction via crossterm's mouse capture:

| Action | Result |
|--------|--------|
| Left click on file | Select/highlight that file |
| Double-click on file | Open file (or enter directory) |
| Left click on menu item | Select and activate menu item |
| Left click on tagged file | Toggle tag |
| Scroll wheel | Scroll file list up/down |
| Click in dialog | Interact with dialog buttons |

**Implementation:**
```rust
terminal.enable_raw_mode()?;
execute!(stdout, EnableMouseCapture)?;
// Handle MouseEvent::Down, MouseEvent::Up, MouseEvent::ScrollUp, etc.
```

### 13.2 Command Line Arguments

Support initial path and other options:

```bash
# Start in specific directory
rdos /path/to/directory
rdos ~/Documents

# Start with specific color theme
rdos --theme monochrome

# Start with specific sort mode
rdos --sort name --order desc

# Show version
rdos --version

# Show help
rdos --help
```

**Argument Parsing (using clap):**
```rust
#[derive(Parser)]
#[command(name = "rdos")]
#[command(about = "R-DOS - A retro DOS-style file manager")]
struct Args {
    /// Initial directory path
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Color theme (default, monochrome, blue, custom)
    #[arg(long, default_value = "default")]
    theme: String,

    /// Sort method (name, ext, size, date, none)
    #[arg(long)]
    sort: Option<String>,

    /// Sort order (asc, desc)
    #[arg(long, default_value = "asc")]
    order: String,
}
```

### 13.3 Responsive Table Layout

The file name column expands to fill available terminal width:

```rust
// Fixed columns
let size_col: u16 = 12;
let date_col: u16 = 10;
let time_col: u16 = 9;
let left_panel: u16 = 30;

// Responsive name column
let separators = 4 + 1; // 4 column separators + right border
let fixed_width = left_panel + size_col + date_col + time_col + separators;
let name_col = terminal_width.saturating_sub(fixed_width).max(14);
```

### 13.4 QDCOLOR - Color Theme Changer

Embedded color configuration screen (press `Ctrl+C` or access via menu):

```
                    R-DOS Color Configuration

                              Current Theme: Default

  ┌─────────────────────────────────────────────────────────┐
  │  1. Default         DOS-style blue/white/yellow         │
  │  2. Monochrome      Black and white only                │
  │  3. Blue            Classic blue theme                  │
  │  4. Green           Matrix-style green                  │
  │  5. Amber           Vintage amber monitor               │
  │  6. Custom          Define your own colors              │
  └─────────────────────────────────────────────────────────┘

  Preview:
  ┌──────────────────────────────────────────────────────────┐
  │ Directory  Tag  View  Copy  Move  Find  Erase  Rename   │
  │ PATH >> /home/user                                       │
  │ ═══════════════════════════════════════════════════════ │
  │  example.txt     1,234    1- 2-25    3:45p              │
  └──────────────────────────────────────────────────────────┘

  Press number to select theme, ESC to cancel, Enter to apply
```

**Theme Structure:**
```toml
[theme]
name = "default"

[theme.colors]
background = "#000000"
foreground = "#FFFFFF"
blue = "#66B7B3"
green = "#67CC4D"
red = "#9D1F14"
yellow = "#E8DA59"

[theme.elements]
menu_fg = "foreground"
menu_bg = "background"
menu_selected_fg = "yellow"
menu_selected_bg = "red"
path_fg = "yellow"
path_bg = "red"
table_header_fg = "blue"
table_border_fg = "foreground"
file_normal_fg = "foreground"
file_directory_fg = "blue"
file_selected_fg = "yellow"
file_selected_bg = "red"
file_tagged_fg = "yellow"
copyright_fg = "blue"
```

### 13.5 QDSTART - Configuration Subprogram

Embedded settings screen (press `Ctrl+S` or access via menu):

```
                    R-DOS Configuration

  ┌─────────────────────────────────────────────────────────┐
  │                                                          │
  │  Search Specification ======>  *.*                       │
  │                                                          │
  │  Sort Method =============>    Name                      │
  │                                                          │
  │  Sort Direction ==========>    Ascending                 │
  │                                                          │
  │  Show Hidden Files =======>    No                        │
  │                                                          │
  │  Confirm Delete ==========>    Yes                       │
  │                                                          │
  │  Default Editor ==========>    $EDITOR                   │
  │                                                          │
  │  Color Theme =============>    Default                   │
  │                                                          │
  │  Mouse Support ===========>    Yes                       │
  │                                                          │
  └─────────────────────────────────────────────────────────┘

  Use arrow keys to select, Space to change, Enter to save, ESC to cancel
```

**Configuration Options:**

| Setting | Values | Default | Description |
|---------|--------|---------|-------------|
| search_spec | glob pattern | `*.*` | File filter pattern |
| sort_method | name/ext/size/date/none | name | How to sort files |
| sort_direction | asc/desc | asc | Sort order |
| show_hidden | yes/no | no | Show dotfiles |
| confirm_delete | yes/no | yes | Confirm before delete |
| default_editor | path | $EDITOR | Editor for F9 |
| color_theme | name | default | Color theme |
| mouse_support | yes/no | yes | Enable mouse |
| uppercase_names | yes/no | yes | Show names in uppercase |

### 13.6 Q-EDIT - Built-in Text Editor

R-DOS includes an embedded text editor (inspired by Q-EDIT) accessible via F9.

**Launching:**
- F9: Edit highlighted file
- ALT-F9: Create new file (blank screen)
- Can also run standalone: `rdos --edit [filename]`

**Screen Layout:**
```
 Again  Buffer  Copy  Del  Edit  Find  Hex  Jump  Print  Quit  Replace  Set  Tag
═══════════════════════════════════════════════════════════════════════════════════
│                                                                                 │
│  [File content here]                                                           │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
═══════════════════════════════════════════════════════════════════════════════════
 filename.txt    Line: 1    Col: 1    INSERT    [Modified]
```

**Navigation Keys:**
| Key | Action |
|-----|--------|
| HOME | Move cursor to start of line |
| END | Move cursor to end of line |
| CTRL-HOME | Move cursor to top of file |
| CTRL-END | Move cursor to end of last line |
| PgUp/PgDn | Scroll one screen |
| Arrow keys | Normal cursor movement |

**Main Commands (press first letter):**

| Command | Key | Description |
|---------|-----|-------------|
| Again | A | Repeat last Find or Replace |
| Buffer | B | Mark text block for copy buffer |
| Copy | C | Insert buffer contents at cursor |
| Del | D | Delete marked block (to buffer) |
| Edit | E | Enter insert mode (show filename) |
| Find | F | Search for text string |
| Hex | H | Switch to hex mode display |
| Jump | J | Jump to marker, line #, or position |
| Print | P | Print marked block |
| Quit | Q | Save/Exit/Initialize submenu |
| Replace | R | Find and replace text |
| Set | S | Configure indent/tab settings |
| Tag | T | Set a marker (A, B, C, or D) |

**Quit Submenu:**
- **Backup**: Save file, create `.BAK` backup
- **Exit**: Exit editor
- **Initialize**: Start editing another file
- **Write**: Save file (overwrite, no backup)

**Jump Submenu:**
- Jump to marker A, B, C, or D
- Jump to line number
- Jump to byte position

**Set Submenu:**
- **Indent**: Toggle auto-indent on/off
- **Tab-size**: Set tab width (2-9 columns)

**Hex Mode:**
```
 Offset    00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F   ASCII
═══════════════════════════════════════════════════════════════════════════════════
 00000000  48 65 6C 6C 6F 20 57 6F  72 6C 64 0D 0A 00 00 00   Hello World.....
 00000010  54 68 69 73 20 69 73 20  61 20 74 65 73 74 0D 0A   This is a test..
═══════════════════════════════════════════════════════════════════════════════════
 Char: 'H'  Hex: 48  Dec: 72                              Press F4 to toggle side
```

- F4 toggles cursor between hex and ASCII side
- INS key to enter insert/overwrite mode
- Hex side: enter two-digit hex values (0-9, A-F)
- ASCII side: type characters directly

**Buffer Operations:**
1. Position cursor at start of block
2. Select BUFFER command
3. Move cursor to end of block (shows char count)
4. Press B to capture block
5. Move cursor to destination
6. Press C to copy (can repeat)

**Limitations:**
- Maximum file size: 60KB (61,440 bytes)
- In-memory buffer: 4,000 chars (overflow to temp file)
- No line-length limit (but slows on 10,000+ char lines)

**Find/Replace:**
- CTRL-R recalls previous search string
- Case-sensitive or case-insensitive search
- AGAIN command repeats last search

**Tips:**
- ALT + number (0-255) enters any ASCII character
- Use Hex mode for faster navigation in binary files
- JUMP POSITION to go to specific byte offset
- Markers (A-D) for quick navigation points

### 13.7 Global Configuration File

Settings stored in `~/.config/rdos/config.toml`:

```toml
# R-DOS Configuration File
# Location: ~/.config/rdos/config.toml

[general]
# Default starting directory (empty = current dir)
default_path = ""

# Show hidden files (dotfiles)
show_hidden = false

# Confirm before destructive operations
confirm_delete = true

# Display file names in uppercase (DOS style)
uppercase_names = true

# Enable mouse support
mouse_support = true

[display]
# Search/filter specification
search_spec = "*.*"

# Sort method: name, ext, size, date, none
sort_method = "name"

# Sort direction: asc, desc
sort_direction = "asc"

# Color theme name
theme = "default"

[editor]
# External editor command (uses $EDITOR if empty)
command = ""

# Arguments to pass to editor
args = []

[keybindings]
# Custom keybinding overrides (optional)
# quit = "q"
# help = "F1"
# tag = "Space"

[themes.default]
background = "#000000"
foreground = "#FFFFFF"
blue = "#66B7B3"
green = "#67CC4D"
red = "#9D1F14"
yellow = "#E8DA59"

[themes.monochrome]
background = "#000000"
foreground = "#AAAAAA"
blue = "#AAAAAA"
green = "#AAAAAA"
red = "#555555"
yellow = "#FFFFFF"

[themes.amber]
background = "#000000"
foreground = "#FFB000"
blue = "#FF8000"
green = "#FFB000"
red = "#804000"
yellow = "#FFFF00"
```

**Config Loading Priority:**
1. Command line arguments (highest)
2. Environment variables (`RDOS_THEME`, `RDOS_EDITOR`, etc.)
3. User config file (`~/.config/rdos/config.toml`)
4. System config (`/etc/rdos/config.toml`)
5. Built-in defaults (lowest)

**Config Directory Structure:**
```
~/.config/rdos/
├── config.toml      # Main configuration
├── themes/          # Custom theme files
│   ├── mytheme.toml
│   └── work.toml
└── history          # Command/path history (optional)
```

---

## 14. Git Integration

R-DOS provides integrated Git support for repositories, including status display, file history, and common operations.

### 14.1 File List Git Status

Files in Git repositories show status indicators in the file name column (flush right):

| Indicator | Color   | Meaning |
|-----------|---------|---------|
| `M`       | Yellow  | Modified (staged or unstaged) |
| `A`       | Cyan    | Added (staged) |
| `D`       | Red     | Deleted |
| `R`       | Cyan    | Renamed |
| `?`       | Magenta | Untracked |
| `!`       | Grey    | Ignored |
| `C`       | Bold Red| Conflict (merge conflict) |
| ` `       | -       | Clean/unchanged |

Directories containing modified files are also marked as modified.

### 14.2 Git Menu

New top-level menu item `Git` (press `G`) for repository operations:

```
Directory  Tag  View  Copy  Move  Find  Erase  Rename  Space  Attribute  Print  Git
Git operations and repository information
```

**Git Submenu:**

| Item | Description |
|------|-------------|
| Status | Show `git status` output |
| Log | Show commit history |
| Diff | Show unstaged changes |
| Staged | Show staged changes |
| Commit | Commit staged changes (prompts for message) |
| Add | Stage highlighted/tagged files |
| Reset | Unstage highlighted/tagged files |
| Stash | Stash/pop/list stashed changes |
| Branch | List/switch/create branches |
| Pull | Pull from remote |
| Push | Push to remote |
| Fetch | Fetch from remote |

**Git Status Screen:**
```
                         R-DOS Git Status
═══════════════════════════════════════════════════════════════════════════════
 Repository: /home/user/project
 Branch: main ↑2 (ahead of origin/main by 2 commits)

 Staged changes:
   M  src/app.rs
   A  src/new_file.rs

 Unstaged changes:
   M  README.md
   M  Cargo.toml

 Untracked files:
   ?  temp.txt
   ?  notes/

═══════════════════════════════════════════════════════════════════════════════
 (A)dd  (R)eset  (C)ommit  (D)iff  (P)ull  (U)push  ESC to return
```

**Git Log Screen:**
```
                         R-DOS Git Log
═══════════════════════════════════════════════════════════════════════════════
 Repository: /home/user/project
 Branch: main

 abc1234  2 hours ago   John Doe      Add new feature
 def5678  5 hours ago   Jane Smith    Fix bug in parser
 ghi9012  1 day ago     John Doe      Update dependencies
 jkl3456  2 days ago    Jane Smith    Initial commit

═══════════════════════════════════════════════════════════════════════════════
 ↑↓ Navigate  ENTER View commit  D Diff  C Checkout  ESC to return
```

### 14.3 File Viewer Git Integration

When viewing a file in a Git repository, additional modes are available:

**View Modes:**
| Key | Mode | Description |
|-----|------|-------------|
| N/A | Normal | Plain text view |
| H | Hex | Hexadecimal view |
| I | Image | Image preview (if supported) |
| M | Markdown | Rendered markdown |
| G | Git History | Show file commit history |
| B | Blame | Git blame annotations |
| D | Diff | Diff against HEAD |

**Git History View (press G):**
```
 VIEW: README.MD  Mode: GIT HISTORY
═══════════════════════════════════════════════════════════════════════════════
 Commit   Date         Author        Message
 ─────────────────────────────────────────────────────────────────────────────
 abc1234  2 hours ago  John Doe      Update documentation
 def5678  1 day ago    Jane Smith    Add installation section
 ghi9012  3 days ago   John Doe      Initial README

═══════════════════════════════════════════════════════════════════════════════
 ↑↓ Navigate  ENTER View version  D Diff with current  < Older  > Newer  ESC
```

**Navigation Between Versions:**
- `<` or `[` — View previous (older) version
- `>` or `]` — View next (newer) version
- `ENTER` on history — View that specific version
- `D` — Toggle diff view comparing selected version with current

**Git Blame View (press B):**
```
 VIEW: README.MD  Mode: BLAME
═══════════════════════════════════════════════════════════════════════════════
 abc1234 John Doe   2h  │ # R-DOS
 abc1234 John Doe   2h  │
 def5678 Jane Smith 1d  │ A retro DOS-style file manager written in Rust.
 def5678 Jane Smith 1d  │
 ghi9012 John Doe   3d  │ ## Installation
 ghi9012 John Doe   3d  │
 ghi9012 John Doe   3d  │ ```bash
 ghi9012 John Doe   3d  │ cargo install rdos
 ghi9012 John Doe   3d  │ ```

═══════════════════════════════════════════════════════════════════════════════
 ↑↓ Scroll  ENTER View commit  N Normal view  ESC exit
```

**Diff View (press D):**
```
 VIEW: README.MD  Mode: DIFF (HEAD)
═══════════════════════════════════════════════════════════════════════════════
 @@ -1,5 +1,7 @@
  # R-DOS

- A file manager written in Rust.
+ A retro DOS-style file manager written in Rust.
+
+ Inspired by Q-DOS II from 1986.

  ## Installation

═══════════════════════════════════════════════════════════════════════════════
 ↑↓ Scroll  C Compare versions  N Normal view  ESC exit
```

**Diff Color Scheme:**
| Element | Color |
|---------|-------|
| Added lines (`+`) | Green |
| Removed lines (`-`) | Red |
| Context lines | White |
| Hunk headers (`@@`) | Cyan |
| File headers | Blue |

### 14.4 Implementation Notes

**Git Detection:**
```rust
fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn get_git_root(path: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;
    // ...
}
```

**File History:**
```rust
fn get_file_history(path: &Path) -> Vec<Commit> {
    let output = Command::new("git")
        .args(["log", "--pretty=format:%h|%ar|%an|%s", "--", path.to_str()?])
        .output()?;
    // Parse output into Commit structs
}
```

**View Specific Version:**
```rust
fn get_file_at_commit(path: &Path, commit: &str) -> Result<Vec<u8>> {
    let relative_path = get_relative_to_git_root(path)?;
    let output = Command::new("git")
        .args(["show", &format!("{}:{}", commit, relative_path)])
        .output()?;
    Ok(output.stdout)
}
```

**Diff Generation:**
```rust
fn get_file_diff(path: &Path, commit: Option<&str>) -> Result<String> {
    let mut args = vec!["diff"];
    if let Some(c) = commit {
        args.push(c);
    }
    args.push("--");
    args.push(path.to_str()?);

    let output = Command::new("git").args(&args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### 14.5 Git Menu Keybindings

When Git menu is active:

| Key | Action |
|-----|--------|
| S | Git Status |
| L | Git Log |
| D | Git Diff (unstaged) |
| T | Git Diff (staged) |
| C | Commit (opens message prompt) |
| A | Add (stage) files |
| R | Reset (unstage) files |
| H | Stash submenu |
| B | Branch submenu |
| P | Pull |
| U | Push |
| F | Fetch |

### 14.6 Planned Git Features

- [ ] Git Status screen with interactive staging
- [ ] Git Log viewer with commit details
- [ ] File history navigation in viewer
- [ ] Git Blame view
- [ ] Diff view with syntax highlighting
- [ ] Branch management (create, switch, delete)
- [ ] Stash operations
- [ ] Pull/Push with remote selection
- [ ] Merge conflict resolution helper
- [ ] Git configuration viewer/editor
- [ ] Submodule support
- [ ] Tag management

---

## 15. Beads Integration

R-DOS provides integrated support for the Beads issue tracker, allowing project management directly from the file manager.

### 15.1 Beads Detection

When navigating to a directory containing a `.beads/` folder, R-DOS automatically detects it as a Beads-enabled project and shows an indicator in the status bar:

```
 C:\PROJECT                           [BEADS: 5 open, 2 ready]    12 Files
```

### 15.2 Beads Menu

New top-level menu item `Beads` (press `B`) for issue tracking operations:

```
Directory  Tag  View  Copy  Move  Find  Erase  Rename  Space  Attribute  Print  Git  Beads
Issue tracking and project management
```

**Beads Submenu:**

| Item | Description |
|------|-------------|
| List | Show all issues (with filters) |
| Ready | Show ready-to-work issues |
| Blocked | Show blocked issues |
| Create | Create new issue |
| Stats | Show project statistics |
| Sync | Sync with git remote |

### 15.3 Issue List Screen

**Main issue list (press L or enter Beads menu):**
```
                         R-DOS Beads Issues
═══════════════════════════════════════════════════════════════════════════════
 Project: /home/user/project                              Filter: [open] [all]

 ID         Pri  Type     Status       Title
 ─────────────────────────────────────────────────────────────────────────────
 PROJ-abc   P1   bug      in_progress  Fix critical login issue
 PROJ-def   P2   feature  open         Add user preferences
 PROJ-ghi   P2   task     open         Update documentation
 PROJ-jkl   P3   feature  blocked      Implement dark mode
 PROJ-mno   P4   task     open         Cleanup unused code

═══════════════════════════════════════════════════════════════════════════════
 ↑↓ Navigate  ENTER Details  N New  F Filter  R Ready  B Blocked  S Sync  ESC
```

**Color coding:**
| Element | Color |
|---------|-------|
| P0/P1 (Critical/High) | Red |
| P2 (Medium) | Yellow |
| P3 (Low) | White |
| P4 (Backlog) | Grey |
| in_progress | Cyan |
| blocked | Magenta |
| closed | Grey/strikethrough |

### 15.4 Issue Detail Screen

**View issue details (press ENTER on issue):**
```
                         R-DOS Issue Details
═══════════════════════════════════════════════════════════════════════════════
 PROJ-abc: Fix critical login issue

 Status: in_progress          Priority: P1 (High)
 Type: bug                    Assignee: thrashr888
 Created: 2026-01-02          Updated: 2026-01-02

 ───────────────────────────────────────────────────────────────────────────────
 Description:

 Users are unable to log in when using special characters in their password.
 This affects all authentication flows.

 ───────────────────────────────────────────────────────────────────────────────
 Dependencies:
   Blocks: PROJ-jkl (Implement dark mode)

 ───────────────────────────────────────────────────────────────────────────────
 Comments (2):
   [2026-01-02 10:30] thrashr888: Started investigation
   [2026-01-02 14:15] thrashr888: Found root cause in auth.rs

═══════════════════════════════════════════════════════════════════════════════
 E Edit  S Status  P Priority  A Assign  C Comment  D Dependencies  X Close  ESC
```

### 15.5 Create Issue Screen

**Create new issue (press N):**
```
                         R-DOS Create Issue
═══════════════════════════════════════════════════════════════════════════════

 Title: _______________________________________________

 Type:     ( ) task    (•) feature    ( ) bug    ( ) epic

 Priority: ( ) P0-Critical  ( ) P1-High  (•) P2-Medium  ( ) P3-Low  ( ) P4-Backlog

 Description:
 ┌─────────────────────────────────────────────────────────────────────────────┐
 │                                                                             │
 │                                                                             │
 │                                                                             │
 └─────────────────────────────────────────────────────────────────────────────┘

 Assignee: [none]                    Parent Epic: [none]

═══════════════════════════════════════════════════════════════════════════════
 TAB Next field  ENTER Create  ESC Cancel
```

### 15.6 Ready/Blocked Views

**Ready issues (press R):**
```
                         R-DOS Ready Issues
═══════════════════════════════════════════════════════════════════════════════
 📋 Ready work (3 issues with no blockers):

 1. [P1] [bug]     PROJ-abc: Fix critical login issue
 2. [P2] [feature] PROJ-def: Add user preferences
 3. [P2] [task]    PROJ-ghi: Update documentation

═══════════════════════════════════════════════════════════════════════════════
 ↑↓ Navigate  ENTER Start working (set in_progress)  ESC back
```

**Blocked issues (press B in Beads menu):**
```
                         R-DOS Blocked Issues
═══════════════════════════════════════════════════════════════════════════════
 🚫 Blocked issues (2):

 [P3] PROJ-jkl: Implement dark mode
   Blocked by: PROJ-abc (Fix critical login issue)

 [P2] PROJ-xyz: Deploy to production
   Blocked by: PROJ-def (Add user preferences), PROJ-ghi (Update docs)

═══════════════════════════════════════════════════════════════════════════════
 ↑↓ Navigate  ENTER View blocker  ESC back
```

### 15.7 Project Statistics

**Stats screen (press S in Beads menu):**
```
                         R-DOS Project Stats
═══════════════════════════════════════════════════════════════════════════════
 Project: my-project

 Issues by Status:              Issues by Type:
 ├─ Open:        12             ├─ Features:  8
 ├─ In Progress:  3             ├─ Bugs:      4
 ├─ Blocked:      2             ├─ Tasks:     5
 └─ Closed:      45             └─ Epics:     2

 Issues by Priority:            Recent Activity:
 ├─ P0 Critical:  0             ├─ Created today:    2
 ├─ P1 High:      2             ├─ Updated today:    5
 ├─ P2 Medium:    8             └─ Closed this week: 3
 ├─ P3 Low:       5
 └─ P4 Backlog:   2

 Epics Progress:
 ├─ Git Integration [████████░░] 80% (8/10)
 └─ UI Improvements [███░░░░░░░] 30% (3/10)

═══════════════════════════════════════════════════════════════════════════════
 ESC to return
```

### 15.8 Quick Actions

**From file list, with issue context:**
- When viewing files, issues mentioning the current file/directory are highlighted
- Press `I` on a file to see related issues
- Quick status updates without leaving file browser

**Keyboard shortcuts in Beads screens:**

| Key | Action |
|-----|--------|
| N | Create new issue |
| E | Edit selected issue |
| S | Change status (cycle: open → in_progress → closed) |
| P | Change priority |
| A | Assign/unassign |
| C | Add comment |
| D | Manage dependencies |
| X | Close issue |
| Y | Sync with remote |
| / | Search/filter issues |

### 15.9 Implementation Notes

**Beads Detection:**
```rust
fn has_beads(path: &Path) -> bool {
    path.join(".beads").is_dir()
}

fn get_beads_stats(path: &Path) -> Option<BeadsStats> {
    let output = Command::new("bd")
        .args(["stats", "--json"])
        .current_dir(path)
        .output()
        .ok()?;
    serde_json::from_slice(&output.stdout).ok()
}
```

**Issue Listing:**
```rust
fn get_issues(path: &Path, filter: &IssueFilter) -> Vec<Issue> {
    let mut args = vec!["list", "--json"];
    if let Some(status) = &filter.status {
        args.extend(["--status", status]);
    }
    let output = Command::new("bd")
        .args(&args)
        .current_dir(path)
        .output()?;
    serde_json::from_slice(&output.stdout)?
}
```

**Status Bar Integration:**
```rust
fn render_status_bar(path: &Path) -> String {
    if has_beads(path) {
        let stats = get_beads_stats(path);
        format!("[BEADS: {} open, {} ready]", stats.open, stats.ready)
    } else {
        String::new()
    }
}
```

### 15.10 Beads Menu Keybindings

When Beads menu is active:

| Key | Action |
|-----|--------|
| L | List all issues |
| R | Show ready issues |
| B | Show blocked issues |
| N | Create new issue |
| S | Project statistics |
| Y | Sync with remote |
| E | Show epics |
| / | Search issues |

### 15.11 Planned Beads Features

- [ ] Issue list with filtering and sorting
- [ ] Issue detail view with full information
- [ ] Create/edit issues from TUI
- [ ] Ready/blocked issue views
- [ ] Project statistics dashboard
- [ ] Sync operations
- [ ] Epic progress visualization
- [ ] Dependency graph view
- [ ] Quick status cycling
- [ ] Comment management
- [ ] File-to-issue linking
- [ ] Search across issues
