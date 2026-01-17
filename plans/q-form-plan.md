# Q-FORM: Form Builder Plan

## Summary

Create a new plugin crate `qdos-plugin-qform` - a form builder and data entry tool for creating and filling out forms. Inspired by Microsoft Access forms, Google Forms, and Filemaker.

## Key Features

1. **Form Designer** - Visual field layout editor
2. **Field Types** - Text, number, date, choice, checkbox, file reference
3. **Validation** - Required, patterns, min/max, custom rules
4. **Data Entry** - Tab through fields, validation feedback
5. **Templates** - Pre-built form templates
6. **Export** - CSV, JSON data export

## Dependencies

```toml
[dependencies]
qdos-plugin-api = { path = "../qdos-plugin-api" }
inventory = "0.3"
ratatui = "0.29"
crossterm = "0.28"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
csv = "1.3"
chrono = "0.4"
dirs = "6.0"
```

## Crate Structure

```
crates/qdos-plugin-qform/
├── Cargo.toml
└── src/
    ├── lib.rs          # Plugin struct, trait impl, key handlers
    ├── state.rs        # QFormState, Field types, Form structure
    ├── modal.rs        # UI rendering (designer, entry, list)
    ├── designer.rs     # Form design operations
    ├── validation.rs   # Field validation logic
    ├── templates.rs    # Built-in form templates
    └── export.rs       # CSV/JSON export
```

## State Design (state.rs)

```rust
pub enum QFormView {
    FormList,       // List of saved forms
    Designer,       // Design mode - edit form structure
    Entry,          // Data entry mode
    Records,        // View submitted records
    Export,         // Export dialog
    Help,
}

pub enum FieldType {
    Text { max_length: Option<usize> },
    Number { min: Option<f64>, max: Option<f64>, decimals: u8 },
    Date { format: String },
    Choice { options: Vec<String>, multi: bool },
    Checkbox,
    FileRef,        // Path to file
    TextArea { rows: u8 },
}

pub struct ValidationRule {
    pub required: bool,
    pub pattern: Option<String>,    // Regex pattern
    pub message: String,            // Error message
}

pub struct Field {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    pub default: Option<String>,
    pub validation: ValidationRule,
    pub help_text: Option<String>,
    pub width: u8,                  // Column span (1-4)
}

pub struct Form {
    pub id: String,
    pub title: String,
    pub description: String,
    pub fields: Vec<Field>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

pub struct Record {
    pub id: String,
    pub form_id: String,
    pub data: HashMap<String, String>,  // field_id -> value
    pub submitted: DateTime<Utc>,
}

pub struct QFormState {
    pub view: QFormView,

    // Forms
    pub forms: Vec<Form>,
    pub current_form: Option<usize>,
    pub form_cursor: usize,

    // Designer
    pub designer_field: usize,      // Selected field index
    pub designer_mode: DesignerMode,
    pub field_edit_buffer: String,

    // Entry
    pub entry_field: usize,         // Current field
    pub entry_values: HashMap<String, String>,
    pub entry_errors: HashMap<String, String>,

    // Records
    pub records: Vec<Record>,
    pub record_cursor: usize,
    pub record_scroll: usize,

    // Export
    pub export_format: ExportFormat,
    pub export_path: String,

    // File
    pub file_path: Option<PathBuf>,
    pub modified: bool,
}

pub enum DesignerMode {
    Navigate,       // Moving between fields
    EditLabel,      // Editing field label
    EditType,       // Changing field type
    EditOptions,    // Editing choice options
    AddField,       // Adding new field
}

pub enum ExportFormat {
    Csv,
    Json,
}
```

## Views

### Form List
```
╔═════════════════════════ Q-FORM ══════════════════════════════════╗
║                                                                   ║
║   YOUR FORMS                                                      ║
║   ──────────                                                      ║
║   [>] Contact Form          8 fields    15 records                ║
║   [ ] Bug Report            12 fields   42 records                ║
║   [ ] Event Registration    10 fields   8 records                 ║
║   [ ] Feedback Survey       6 fields    23 records                ║
║                                                                   ║
║                                                                   ║
║                                                                   ║
╠═══════════════════════════════════════════════════════════════════╣
║ 4 forms                                                           ║
╚═══════════════════════════════════════════════════════════════════╝
 N:New  Enter:Open  D:Design  E:Entry  R:Records  X:Export  Esc:Exit
```

### Form Designer
```
╔════════════════ Q-FORM: Contact Form (Design) ════════════════════╗
║                                                                   ║
║   Field 1: [Name____________]  Text      Required                 ║
║   Field 2: [Email___________]  Text      Required, Email pattern  ║
║   Field 3: [Phone___________]  Text      Optional                 ║
║   Field 4: [Department______]  Choice    Required                 ║
║            Options: Sales, Support, Marketing, Other              ║
║   Field 5: [Message_________]  TextArea  Required                 ║
║            (4 rows)                                               ║
║   Field 6: [Subscribe?]        Checkbox  Default: Yes             ║
║                                                                   ║
║   [+ Add Field]                                                   ║
║                                                                   ║
╠═══════════════════════════════════════════════════════════════════╣
║ Editing: Field 2 (Email) - Press E to edit, Del to remove        ║
╚═══════════════════════════════════════════════════════════════════╝
 ↑↓:Navigate  A:Add  E:Edit  T:Type  V:Validation  Del:Remove
```

### Data Entry
```
╔════════════════ Q-FORM: Contact Form (Entry) ═════════════════════╗
║                                                                   ║
║   Name:        [John Doe________________]                         ║
║                                                                   ║
║   Email:       [john@example.com________]                         ║
║                                                                   ║
║   Phone:       [555-1234________________]                         ║
║                                                                   ║
║   Department:  [ ] Sales                                          ║
║                [x] Support                                        ║
║                [ ] Marketing                                      ║
║                [ ] Other                                          ║
║                                                                   ║
║   Message:     [I need help with my order. The tracking          ║
║                 number shows delivered but I haven't              ║
║                 received anything yet.________________]           ║
║                                                                   ║
║   Subscribe?   [x] Yes, send me updates                           ║
║                                                                   ║
╠═══════════════════════════════════════════════════════════════════╣
║ Field 5/6 (Message)                                               ║
╚═══════════════════════════════════════════════════════════════════╝
 Tab:Next  Shift+Tab:Prev  Enter:Submit  Esc:Cancel
```

## Key Bindings

### Form List
| Key | Action |
|-----|--------|
| N | New form |
| Enter | Open selected form |
| D | Design mode |
| E | Entry mode |
| R | View records |
| X | Export records |
| Del | Delete form |
| Esc | Exit |

### Designer
| Key | Action |
|-----|--------|
| ↑↓ | Navigate fields |
| A | Add field |
| E | Edit label |
| T | Change type |
| V | Edit validation |
| O | Edit options (for Choice) |
| Del | Remove field |
| Ctrl+↑↓ | Reorder fields |
| Ctrl+S | Save form |
| Esc | Back to list |

### Entry
| Key | Action |
|-----|--------|
| Tab | Next field |
| Shift+Tab | Previous field |
| Enter | Submit (on last field) / Toggle (checkbox) |
| Space | Toggle (checkbox/choice) |
| ↑↓ | Navigate choices |
| Esc | Cancel |

## Implementation Phases

### Phase 1: Core Structure
1. Create crate skeleton with Cargo.toml
2. Implement state types (Form, Field, Record)
3. Implement Plugin trait boilerplate
4. Add to workspace Cargo.toml

### Phase 2: Form List
1. Form list rendering
2. Form persistence (JSON file)
3. Create new form dialog

### Phase 3: Designer
1. Field list rendering
2. Add/remove fields
3. Edit field properties
4. Change field types

### Phase 4: Field Types
1. Text input
2. Number input with validation
3. Date input with format
4. Choice (radio/multi-select)
5. Checkbox
6. TextArea

### Phase 5: Data Entry
1. Field-by-field entry
2. Tab navigation
3. Validation on submit
4. Record storage

### Phase 6: Records & Export
1. Records list view
2. CSV export
3. JSON export

### Phase 7: Polish
1. Templates (contact form, survey, registration)
2. Integration with Office Suite
3. Help screen

## File Modifications

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `qdos-plugin-qform` to members |
| `crates/qdos-plugin-qform/*` | **NEW** - All plugin files |
| `src/plugins/mod.rs` | Add import for QFormPlugin |
| `src/app/mod.rs` | Register QFormPlugin |
| `src/plugins/office/mod.rs` | Add to Office Suite menu |

## Data Storage

Forms and records stored in:
```
~/.config/rdos/qform/
├── forms.json          # Form definitions
└── records/
    ├── form-001.json   # Records for form 001
    └── form-002.json   # Records for form 002
```

## Verification

1. `cargo build -p qdos-plugin-qform` - Plugin compiles
2. Create new form with various field types
3. Test validation (required, patterns)
4. Submit records
5. Export to CSV and JSON
6. Quality checks: `cargo fmt -- --check && cargo clippy -- -D warnings`
