//! Help Plugin State Types

/// A help topic with key shortcut, title, and content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpTopic {
    pub key: char,
    pub title: String,
    pub content: String,
}

/// Help system state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpState {
    pub topics: Vec<HelpTopic>,
    pub current_topic: usize, // 0 = index page, 1+ = topic pages
    pub scroll_offset: usize,
}

impl Default for HelpState {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpState {
    pub fn new() -> Self {
        let topics = Self::load_topics();
        Self {
            topics,
            current_topic: 0,
            scroll_offset: 0,
        }
    }

    fn load_topics() -> Vec<HelpTopic> {
        vec![
            HelpTopic {
                key: 'I',
                title: "Introduction to Q-DOS II".to_string(),
                content: r#"Q-DOS II lets you easily manage DOS directories and files. You can
create directories with a few keystrokes and see them displayed on a
"directory map." You can find "lost" files located anywhere on the
disk and you can also edit files in any directory.

Q-DOS II lets you mark files and move, copy, rename, print, or erase
them without ever typing file names. You can load and execute
programs or any DOS commands.

HOW TO SELECT A COMMAND

As you enter Q-DOS II, you will see the Main Screen with the main
commands listed on the top line. One of them will be "highlighted."

You may select a command by highlighting it with the arrow keys
and pressing RETURN, or by pressing the first letter of the command.

HOW TO TAG FILES

COPY, ERASE, RENAME, PRINT, ATTRIBUTE, and MOVE can operate on
several files at once. You identify multiple files by tagging them.
Press SPACE BAR to tag/untag the highlighted file.

THE ESC KEY

The Escape (ESC) key returns you to the Main Screen. When pressed
in the middle of a command, it will cancel the command."#
                    .to_string(),
            },
            HelpTopic {
                key: 'A',
                title: "Attribute Command".to_string(),
                content: r#"The ATTRIBUTE command allows you to display and/or change file
attributes. File attributes include: HID (Hidden), SYS (System),
R/O (Read-Only), and ARC (Archive).

On Unix/macOS, only the R/O (Read-Only) attribute can be modified.
This controls whether the file has write permissions.

TO USE:
1. Highlight a file or tag multiple files
2. Select ATTRIBUTE from the menu
3. Use arrow keys to select an attribute
4. Press SPACE to toggle ON/OFF/N/C
5. Press ENTER to apply changes

Note: You cannot change DIR, NORM, or VOL attributes."#
                    .to_string(),
            },
            HelpTopic {
                key: 'C',
                title: "Copy Command".to_string(),
                content: r#"The COPY command copies files from the current directory to another.

TO USE:
1. Tag files to copy (or use highlighted file)
2. Select COPY from the menu
3. Enter destination path (Tab for completion)
4. Press ENTER to copy

The original files remain in their current location.
Use Tab for path auto-completion."#
                    .to_string(),
            },
            HelpTopic {
                key: 'D',
                title: "Directory Command".to_string(),
                content: r#"The DIRECTORY command lets you manage directories.

DIRECTORY MAP (D key):
Opens a tree view of all directories. You can:
- Navigate with arrow keys
- Expand/collapse with Enter or Right/Left arrows
- Create new directories with M
- Delete empty directories with D (requires confirmation)

CHANGE DIRECTORY (F5):
Enter a path to change to a different directory.

PREVIOUS DIRECTORY (F4):
Return to the previously visited directory."#
                    .to_string(),
            },
            HelpTopic {
                key: 'E',
                title: "Erase Command".to_string(),
                content: r#"The ERASE command deletes files from the current directory.

TO USE:
1. Tag files to erase (or use highlighted file)
2. Select ERASE from the menu
3. Confirm with Y or cancel with N

WARNING: Erased files cannot be recovered!

Use with caution. Consider using a trash/recycle bin instead."#
                    .to_string(),
            },
            HelpTopic {
                key: 'F',
                title: "Find Command".to_string(),
                content: r#"The FIND command searches for files across directories.

TO USE:
1. Select FIND from the menu (or press Ctrl+F)
2. Enter a search pattern (supports wildcards)
3. Results show matching files with paths
4. Select a result to navigate to that file

WILDCARDS:
* - Matches any characters
? - Matches a single character

Example: *.txt finds all text files"#
                    .to_string(),
            },
            HelpTopic {
                key: 'G',
                title: "Git Integration".to_string(),
                content: r#"R-DOS includes Git version control integration.

OPEN GIT MENU: Press G

GIT MENU OPTIONS:
- Status: View changed/staged files
- Log: View commit history
- Diff: View changes in files
- Commit: Create a new commit
- Push/Pull: Sync with remote
- Branch: Switch branches
- Stash: Temporarily save changes

The status bar shows: branch name, ahead/behind counts,
and number of modified files."#
                    .to_string(),
            },
            HelpTopic {
                key: 'B',
                title: "Beads Issue Tracker".to_string(),
                content: r#"R-DOS includes Beads issue tracker integration.

OPEN BEADS MENU: Press B

BEADS MENU OPTIONS:
- List: View all open issues
- Ready: Show issues ready to work
- Blocked: Show blocked issues
- Create: Create a new issue
- Kanban: View issues in kanban board
- Stats: View project statistics
- Sync: Sync with git remote

The status bar shows issue counts when in a beads project."#
                    .to_string(),
            },
            HelpTopic {
                key: 'M',
                title: "Move Command".to_string(),
                content: r#"The MOVE command moves files to another directory.

TO USE:
1. Tag files to move (or use highlighted file)
2. Select MOVE from the menu
3. Enter destination path (Tab for completion)
4. Press ENTER to move

Unlike COPY, the original files are removed after moving."#
                    .to_string(),
            },
            HelpTopic {
                key: 'R',
                title: "Rename Command".to_string(),
                content: r#"The RENAME command changes file names.

TO USE:
1. Highlight a file or tag multiple files
2. Select RENAME from the menu
3. For single file: Enter new name
4. For multiple files: Use pattern replacement

BATCH RENAME:
When renaming multiple files, you can use:
- Find/Replace: Replace text in filenames
- Numbering: Add sequential numbers"#
                    .to_string(),
            },
            HelpTopic {
                key: 'V',
                title: "View Command".to_string(),
                content: r#"The VIEW command displays file contents.

TO USE:
1. Highlight a file
2. Select VIEW or press Enter

VIEW MODES (press key to switch):
- A: ASCII text view
- H: Hexadecimal view
- M: Markdown rendered view
- I: Image view (for supported formats)

NAVIGATION:
- Up/Down: Scroll by line
- PgUp/PgDn: Scroll by page
- Home/End: Go to start/end
- /: Search in file
- ESC: Exit viewer"#
                    .to_string(),
            },
            HelpTopic {
                key: 'K',
                title: "Keyboard Shortcuts".to_string(),
                content: r#"GLOBAL SHORTCUTS:
F1      Help (this screen)
F2      System status
F4      Parent directory
F5      Change directory
F10     Quit
Ctrl+T  Color theme
Ctrl+S  Settings (QDSTART)
Ctrl+R  Refresh file list
Ctrl+F  Find files

NAVIGATION:
↑/↓     Move selection
←/→     Switch panels (if dual-pane)
Enter   Open file/directory
Space   Tag/untag file
Tab     Auto-complete paths

FILE OPERATIONS:
Y       Copy to clipboard
G       Git menu
B       Beads menu
-       Go back in history
=/+     Go forward in history"#
                    .to_string(),
            },
        ]
    }
}
