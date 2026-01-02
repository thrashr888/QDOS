//! Q-DOS II style error messages
//!
//! Error messages matching the original Q-DOS II application.

/// Disk-related error messages
pub mod disk {
    pub const GENERAL_DISK_ERROR: &str = "GENERAL DISK ERROR";
    pub const WRITE_PROTECT_ERROR: &str = "WRITE-PROTECT ERROR";
    pub const DISK_FULL_ERROR: &str = "DISK FULL ERROR";
    pub const DRIVE_NOT_READY: &str = "DRIVE IS NOT READY";

    pub fn disk_full_insert(drive: &str) -> String {
        format!(
            "DISK {} IS FULL -- INSERT NEW DISK AND (C)ONTINUE OR (S)KIP FILE",
            drive.to_uppercase()
        )
    }

    pub const DISK_CHANGED: &str = "DISK HAS BEEN CHANGED -- press any key to go to main menu";
}

/// I/O error messages
pub mod io {
    pub const IO_ERROR: &str = "* * * *   I/O ERROR   * * * *";
    pub const PRINTER_IO_ERROR: &str = "Printer I/O error";
    pub const PRINTER_ERROR: &str = "Printer Error";
    pub const ERROR_READING_FILE: &str = "Error reading file. RET to skip or ESC to abort";
}

/// File operation error messages
pub mod file {
    pub const CANNOT_COPY_DIR_FULL: &str = "CANNOT COPY ANY MORE FILES -- DIRECTORY IS FULL";
    pub const CANNOT_OPEN_SOURCE: &str =
        "CANNOT OPEN SOURCEFILE; copy terminated. Press any key to continue.";
    pub const CANNOT_OPEN_HIGHLIGHTED: &str = "CANNOT OPEN THE HIGHLIGHTED FILE";
    pub const CANNOT_MOVE_DIR_FULL: &str = "CANNOT MOVE ANY MORE FILES -- DIRECTORY IS FULL";
    pub const CANNOT_FIND_HELP: &str = "CANNOT FIND QDOS.HLP -- PRESS LETTER OF DRIVE";
    pub const CANNOT_RENAME: &str = "CAN'T RENAME THE FILE";
    pub const CANNOT_RENAME_READONLY: &str = "CAN'T RENAME THE FILE -- IT IS MARKED 'READ-ONLY'";
    pub const CANNOT_RENAME_DUPLICATE: &str =
        "CAN'T RENAME THE FILE -- THERE IS ANOTHER ONE OF THE SAME NAME";
    pub const CANNOT_ERASE_DIR: &str = "CAN'T ERASE DIRECTORIES";
    pub const CANNOT_MOVE_DIR: &str = "CAN'T MOVE DIRECTORIES";
    pub const CANNOT_COPY_DIR: &str = "CAN'T COPY DIRECTORIES";
    pub const CANNOT_VIEW_DIR: &str = "You cannot view a directory";
    pub const FILE_ERASED: &str = "File erased";
    pub const ERROR_ERASE: &str = "Error - unable to erase";
    pub const NO_MATCHING_FILES: &str = "No matching files found.";

    pub fn file_readonly_no_erase(filename: &str) -> String {
        format!(
            "{} IS MARKED READ-ONLY -- CANNOT ERASE IT",
            filename.to_uppercase()
        )
    }

    pub fn file_exists_no_move(filename: &str) -> String {
        format!(
            "{} EXISTS IN DESTINATION DIRECTORY -- CANNOT MOVE IT",
            filename.to_uppercase()
        )
    }

    pub fn file_exists_no_copy(filename: &str, drive: &str) -> String {
        format!(
            "{} EXISTS ON DRIVE {} -- WILL NOT BE COPIED",
            filename.to_uppercase(),
            drive.to_uppercase()
        )
    }

    pub const DEST_READONLY_OVERWRITE: &str = "DESTINATION FILE OF SAME NAME IS R/O - OVERWRITE?";
    pub const FILE_TOO_LARGE: &str = "File too large -- cannot edit the file";
}

/// File attribute error messages
pub mod attr {
    pub const CANNOT_CHANGE_DIR_NORM_VOL: &str =
        "You can't change DIR, NORM, or VOL attributes";
}

/// Path and directory error messages
pub mod path {
    pub const PATH_NOT_AVAILABLE: &str = "PATH (OR FILE) NOT AVAILABLE";
    pub const PATH_NOT_AVAILABLE_DISK_CHANGED: &str =
        "PATH (OR FILE) NOT AVAILABLE -- YOU MUST HAVE CHANGED DISKS";
    pub const NAME_INVALID_DIR_EXISTS: &str =
        "THAT NAME IS INVALID -- THERE IS ALREADY A DIRECTORY BY THAT NAME";
    pub const DEST_SAME_AS_SOURCE: &str = "THE DESTINATION IS THE SAME AS THE SOURCE";
    pub const DIR_NOT_EMPTY: &str = "THE DIRECTORY IS NOT EMPTY";
    pub const CANNOT_REMOVE_ROOT: &str = "THE ROOT DIRECTORY CANNOT BE REMOVED";
    pub const NO_TAGGED_FILES: &str = "THERE ARE NO TAGGED FILES IN THIS DIRECTORY";
    pub const MUST_CHANGE_DRIVE: &str = "YOU MUST CHANGE TO A DRIVE FIRST";
}

/// Memory and resource error messages
pub mod memory {
    pub const FILE_NOT_FOUND_OR_NO_MEMORY: &str = "FILE NOT FOUND OR NOT ENOUGH MEMORY";
    pub const NOT_ENOUGH_MEMORY: &str = "THERE IS NOT ENOUGH MEMORY TO RUN THIS PROGRAM";
    pub const NOT_ENOUGH_SPACE_DIR: &str = "NOT ENOUGH SPACE TO READ IN ENTIRE DIRECTORY";
    pub const TOO_MANY_DIRS: &str = "TOO MANY DIRECTORIES";
}

/// Command and operation error messages
pub mod command {
    pub const NO_FILES_FOR_COMMAND: &str = "THAT COMMAND WON'T WORK WHEN THERE ARE NO FILES";
    pub const NO_FILES_IN_DIR: &str = "NO FILES IN DIRECTORY";
    pub const ERROR_EXECUTING: &str = "Error Executing";

    pub fn error_executing(cmd: &str) -> String {
        format!("Error Executing: {}", cmd)
    }
}

/// Drive error messages
pub mod drive {
    pub const INVALID_DRIVE: &str = "Invalid drive.";
    pub const SPECIFIED_DRIVE_INVALID: &str = "Specified drive is invalid.";
    pub const NO_DRIVES_AVAILABLE: &str = "No drives available.  Unable to run Q-DOS.";
}

/// Success messages (for consistency)
pub mod success {
    pub const FILES_COPIED: &str = "File(s) copied successfully.";
    pub const FILES_MOVED: &str = "File(s) moved successfully.";
    pub const FILES_ERASED: &str = "File(s) erased.";
    pub const FILE_RENAMED: &str = "File renamed successfully.";
    pub const DIR_CREATED: &str = "Directory created.";
    pub const DIR_REMOVED: &str = "Directory removed.";
}

/// Confirmation messages
pub mod confirm {
    pub const PRESS_ANY_KEY: &str = "Press any key to continue";
    pub const PRESS_ESC_ABORT: &str = "Press ESC to abort";

    pub fn erase_confirm(count: usize) -> String {
        if count == 1 {
            "Erase this file? (Y/N)".to_string()
        } else {
            format!("Erase {} files? (Y/N)", count)
        }
    }

    pub fn copy_confirm(count: usize, dest: &str) -> String {
        if count == 1 {
            format!("Copy file to {}? (Y/N)", dest)
        } else {
            format!("Copy {} files to {}? (Y/N)", count, dest)
        }
    }

    pub fn move_confirm(count: usize, dest: &str) -> String {
        if count == 1 {
            format!("Move file to {}? (Y/N)", dest)
        } else {
            format!("Move {} files to {}? (Y/N)", count, dest)
        }
    }
}
