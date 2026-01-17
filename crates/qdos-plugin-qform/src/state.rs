//! Q-FORM state and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// =============================================================================
// VIEWS
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QFormView {
    #[default]
    FormList,
    Designer,
    Entry,
    Records,
    Export,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DesignerMode {
    #[default]
    Navigate,
    EditLabel,
    EditType,
    EditOptions,
    AddField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Csv,
    Json,
}

// =============================================================================
// FIELD TYPES
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    Text {
        max_length: Option<usize>,
    },
    Number {
        min: Option<f64>,
        max: Option<f64>,
        decimals: u8,
    },
    Date {
        format: String,
    },
    Choice {
        options: Vec<String>,
        multi: bool,
    },
    Checkbox,
    FileRef,
    TextArea {
        rows: u8,
    },
}

impl Default for FieldType {
    fn default() -> Self {
        Self::Text { max_length: None }
    }
}

impl FieldType {
    pub fn name(&self) -> &'static str {
        match self {
            FieldType::Text { .. } => "Text",
            FieldType::Number { .. } => "Number",
            FieldType::Date { .. } => "Date",
            FieldType::Choice { multi: false, .. } => "Choice",
            FieldType::Choice { multi: true, .. } => "Multi-Choice",
            FieldType::Checkbox => "Checkbox",
            FieldType::FileRef => "File",
            FieldType::TextArea { .. } => "Text Area",
        }
    }

    pub fn all_types() -> Vec<FieldType> {
        vec![
            FieldType::Text { max_length: None },
            FieldType::Number {
                min: None,
                max: None,
                decimals: 2,
            },
            FieldType::Date {
                format: "%Y-%m-%d".to_string(),
            },
            FieldType::Choice {
                options: vec!["Option 1".to_string(), "Option 2".to_string()],
                multi: false,
            },
            FieldType::Choice {
                options: vec!["Option 1".to_string(), "Option 2".to_string()],
                multi: true,
            },
            FieldType::Checkbox,
            FieldType::FileRef,
            FieldType::TextArea { rows: 4 },
        ]
    }
}

// =============================================================================
// VALIDATION
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationRule {
    pub required: bool,
    pub pattern: Option<String>,
    pub message: String,
}

// =============================================================================
// FIELD & FORM
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    pub default: Option<String>,
    pub validation: ValidationRule,
    pub help_text: Option<String>,
    pub width: u8,
}

impl Field {
    pub fn new(id: &str, label: &str, field_type: FieldType) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            field_type,
            default: None,
            validation: ValidationRule::default(),
            help_text: None,
            width: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Form {
    pub id: String,
    pub title: String,
    pub description: String,
    pub fields: Vec<Field>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

impl Form {
    pub fn new(title: &str) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        Self {
            id,
            title: title.to_string(),
            description: String::new(),
            fields: Vec::new(),
            created: now,
            modified: now,
        }
    }
}

// =============================================================================
// RECORD
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub form_id: String,
    pub data: HashMap<String, String>,
    pub submitted: DateTime<Utc>,
}

impl Record {
    pub fn new(form_id: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            form_id: form_id.to_string(),
            data: HashMap::new(),
            submitted: Utc::now(),
        }
    }
}

// =============================================================================
// STATE
// =============================================================================

#[derive(Debug)]
pub struct QFormState {
    pub view: QFormView,

    // Forms
    pub forms: Vec<Form>,
    pub current_form: Option<usize>,
    pub form_cursor: usize,

    // Designer
    pub designer_field: usize,
    pub designer_mode: DesignerMode,
    pub field_edit_buffer: String,
    pub type_cursor: usize,

    // Entry
    pub entry_field: usize,
    pub entry_values: HashMap<String, String>,
    pub entry_errors: HashMap<String, String>,
    pub choice_cursor: usize,

    // Records
    pub records: Vec<Record>,
    pub record_cursor: usize,
    pub record_scroll: usize,

    // Export
    pub export_format: ExportFormat,
    pub export_path: String,

    // File paths
    pub forms_path: PathBuf,
    pub records_path: PathBuf,

    // State
    pub modified: bool,
    pub status_message: Option<String>,
}

impl Default for QFormState {
    fn default() -> Self {
        Self::new()
    }
}

impl QFormState {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rdos")
            .join("qform");

        Self {
            view: QFormView::FormList,
            forms: Vec::new(),
            current_form: None,
            form_cursor: 0,
            designer_field: 0,
            designer_mode: DesignerMode::Navigate,
            field_edit_buffer: String::new(),
            type_cursor: 0,
            entry_field: 0,
            entry_values: HashMap::new(),
            entry_errors: HashMap::new(),
            choice_cursor: 0,
            records: Vec::new(),
            record_cursor: 0,
            record_scroll: 0,
            export_format: ExportFormat::Csv,
            export_path: String::new(),
            forms_path: config_dir.join("forms.json"),
            records_path: config_dir.join("records"),
            modified: false,
            status_message: None,
        }
    }

    // =========================================================================
    // FORM OPERATIONS
    // =========================================================================

    pub fn current_form(&self) -> Option<&Form> {
        self.current_form.and_then(|i| self.forms.get(i))
    }

    pub fn current_form_mut(&mut self) -> Option<&mut Form> {
        self.current_form.and_then(|i| self.forms.get_mut(i))
    }

    pub fn create_form(&mut self, title: &str) {
        let form = Form::new(title);
        self.forms.push(form);
        self.current_form = Some(self.forms.len() - 1);
        self.modified = true;
    }

    pub fn delete_form(&mut self, index: usize) {
        if index < self.forms.len() {
            self.forms.remove(index);
            self.current_form = None;
            if self.form_cursor >= self.forms.len() && self.form_cursor > 0 {
                self.form_cursor -= 1;
            }
            self.modified = true;
        }
    }

    pub fn select_form(&mut self, index: usize) {
        if index < self.forms.len() {
            self.current_form = Some(index);
        }
    }

    // =========================================================================
    // FIELD OPERATIONS
    // =========================================================================

    pub fn add_field(&mut self, label: &str, field_type: FieldType) {
        let form_idx = match self.current_form {
            Some(idx) => idx,
            None => return,
        };
        let id = format!("field_{}", self.forms[form_idx].fields.len() + 1);
        let field = Field::new(&id, label, field_type);
        self.forms[form_idx].fields.push(field);
        self.forms[form_idx].modified = Utc::now();
        self.modified = true;
    }

    pub fn remove_field(&mut self, index: usize) {
        let form_idx = match self.current_form {
            Some(idx) => idx,
            None => return,
        };
        let fields_len = self.forms[form_idx].fields.len();
        if index < fields_len {
            self.forms[form_idx].fields.remove(index);
            self.forms[form_idx].modified = Utc::now();
            self.modified = true;
            let new_len = self.forms[form_idx].fields.len();
            if self.designer_field >= new_len && self.designer_field > 0 {
                self.designer_field -= 1;
            }
        }
    }

    pub fn move_field_up(&mut self) {
        let form_idx = match self.current_form {
            Some(idx) => idx,
            None => return,
        };
        let designer_field = self.designer_field;
        let fields_len = self.forms[form_idx].fields.len();
        if designer_field > 0 && designer_field < fields_len {
            self.forms[form_idx]
                .fields
                .swap(designer_field, designer_field - 1);
            self.designer_field -= 1;
            self.forms[form_idx].modified = Utc::now();
            self.modified = true;
        }
    }

    pub fn move_field_down(&mut self) {
        let form_idx = match self.current_form {
            Some(idx) => idx,
            None => return,
        };
        let designer_field = self.designer_field;
        let fields_len = self.forms[form_idx].fields.len();
        if designer_field + 1 < fields_len {
            self.forms[form_idx]
                .fields
                .swap(designer_field, designer_field + 1);
            self.designer_field += 1;
            self.forms[form_idx].modified = Utc::now();
            self.modified = true;
        }
    }

    // =========================================================================
    // ENTRY OPERATIONS
    // =========================================================================

    pub fn start_entry(&mut self) {
        let form_idx = match self.current_form {
            Some(idx) => idx,
            None => return,
        };

        self.entry_values.clear();
        self.entry_errors.clear();
        self.entry_field = 0;
        self.choice_cursor = 0;

        // Initialize with defaults - collect field info first
        let defaults: Vec<(String, Option<String>, bool)> = self.forms[form_idx]
            .fields
            .iter()
            .map(|f| {
                (
                    f.id.clone(),
                    f.default.clone(),
                    matches!(f.field_type, FieldType::Checkbox),
                )
            })
            .collect();

        for (field_id, default, is_checkbox) in defaults {
            if let Some(default_val) = default {
                self.entry_values.insert(field_id, default_val);
            } else if is_checkbox {
                self.entry_values.insert(field_id, "false".to_string());
            }
        }
    }

    pub fn validate_entry(&mut self) -> bool {
        self.entry_errors.clear();

        let form_idx = match self.current_form {
            Some(idx) => idx,
            None => return true,
        };

        // Collect validation info from fields first
        struct ValidationInfo {
            field_id: String,
            label: String,
            required: bool,
            pattern: Option<String>,
            message: String,
            number_min: Option<f64>,
            number_max: Option<f64>,
            is_number: bool,
        }

        let validations: Vec<ValidationInfo> = self.forms[form_idx]
            .fields
            .iter()
            .map(|f| {
                let (number_min, number_max, is_number) = match &f.field_type {
                    FieldType::Number { min, max, .. } => (*min, *max, true),
                    _ => (None, None, false),
                };
                ValidationInfo {
                    field_id: f.id.clone(),
                    label: f.label.clone(),
                    required: f.validation.required,
                    pattern: f.validation.pattern.clone(),
                    message: f.validation.message.clone(),
                    number_min,
                    number_max,
                    is_number,
                }
            })
            .collect();

        for info in validations {
            let value = self
                .entry_values
                .get(&info.field_id)
                .map(|s| s.as_str())
                .unwrap_or("");

            // Required check
            if info.required && value.is_empty() {
                let msg = if info.message.is_empty() {
                    format!("{} is required", info.label)
                } else {
                    info.message.clone()
                };
                self.entry_errors.insert(info.field_id.clone(), msg);
                continue;
            }

            // Pattern check
            if !value.is_empty() {
                if let Some(pattern) = &info.pattern {
                    // Simple email pattern check
                    if pattern == "email" && !value.contains('@') {
                        self.entry_errors
                            .insert(info.field_id.clone(), "Invalid email address".to_string());
                    }
                }
            }

            // Number validation
            if !value.is_empty() && info.is_number {
                if let Ok(num) = value.parse::<f64>() {
                    if let Some(min_val) = info.number_min {
                        if num < min_val {
                            self.entry_errors.insert(
                                info.field_id.clone(),
                                format!("Value must be at least {}", min_val),
                            );
                        }
                    }
                    if let Some(max_val) = info.number_max {
                        if num > max_val {
                            self.entry_errors.insert(
                                info.field_id.clone(),
                                format!("Value must be at most {}", max_val),
                            );
                        }
                    }
                } else {
                    self.entry_errors
                        .insert(info.field_id.clone(), "Invalid number".to_string());
                }
            }
        }

        self.entry_errors.is_empty()
    }

    pub fn submit_entry(&mut self) -> Result<(), String> {
        if !self.validate_entry() {
            return Err("Validation errors".to_string());
        }

        if let Some(form) = self.current_form() {
            let mut record = Record::new(&form.id);
            record.data = self.entry_values.clone();
            self.records.push(record);
            self.entry_values.clear();
            self.entry_field = 0;
            Ok(())
        } else {
            Err("No form selected".to_string())
        }
    }

    pub fn current_entry_value(&self) -> String {
        if let Some(form) = self.current_form() {
            if let Some(field) = form.fields.get(self.entry_field) {
                return self
                    .entry_values
                    .get(&field.id)
                    .cloned()
                    .unwrap_or_default();
            }
        }
        String::new()
    }

    pub fn set_current_entry_value(&mut self, value: String) {
        if let Some(form) = self.current_form() {
            if let Some(field) = form.fields.get(self.entry_field) {
                self.entry_values.insert(field.id.clone(), value);
            }
        }
    }

    // =========================================================================
    // NAVIGATION
    // =========================================================================

    pub fn form_cursor_up(&mut self) {
        if self.form_cursor > 0 {
            self.form_cursor -= 1;
        }
    }

    pub fn form_cursor_down(&mut self) {
        if self.form_cursor + 1 < self.forms.len() {
            self.form_cursor += 1;
        }
    }

    pub fn designer_cursor_up(&mut self) {
        if self.designer_field > 0 {
            self.designer_field -= 1;
        }
    }

    pub fn designer_cursor_down(&mut self) {
        if let Some(form) = self.current_form() {
            if self.designer_field + 1 < form.fields.len() {
                self.designer_field += 1;
            }
        }
    }

    pub fn entry_prev_field(&mut self) {
        if self.entry_field > 0 {
            self.entry_field -= 1;
            self.choice_cursor = 0;
        }
    }

    pub fn entry_next_field(&mut self) {
        if let Some(form) = self.current_form() {
            if self.entry_field + 1 < form.fields.len() {
                self.entry_field += 1;
                self.choice_cursor = 0;
            }
        }
    }

    pub fn record_cursor_up(&mut self) {
        if self.record_cursor > 0 {
            self.record_cursor -= 1;
        }
    }

    pub fn record_cursor_down(&mut self) {
        let form_records = self.records_for_current_form();
        if self.record_cursor + 1 < form_records.len() {
            self.record_cursor += 1;
        }
    }

    pub fn records_for_current_form(&self) -> Vec<&Record> {
        if let Some(form) = self.current_form() {
            self.records
                .iter()
                .filter(|r| r.form_id == form.id)
                .collect()
        } else {
            Vec::new()
        }
    }
}

// =============================================================================
// TEMPLATES
// =============================================================================

impl QFormState {
    pub fn create_contact_form(&mut self) {
        self.create_form("Contact Form");

        self.add_field(
            "Name",
            FieldType::Text {
                max_length: Some(100),
            },
        );
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
            }
        }

        self.add_field(
            "Email",
            FieldType::Text {
                max_length: Some(255),
            },
        );
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
                field.validation.pattern = Some("email".to_string());
            }
        }

        self.add_field(
            "Phone",
            FieldType::Text {
                max_length: Some(20),
            },
        );

        self.add_field(
            "Department",
            FieldType::Choice {
                options: vec![
                    "Sales".to_string(),
                    "Support".to_string(),
                    "Marketing".to_string(),
                    "Other".to_string(),
                ],
                multi: false,
            },
        );
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
            }
        }

        self.add_field("Message", FieldType::TextArea { rows: 4 });
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
            }
        }

        self.add_field("Subscribe to updates", FieldType::Checkbox);
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.default = Some("true".to_string());
            }
        }
    }

    pub fn create_bug_report_form(&mut self) {
        self.create_form("Bug Report");

        self.add_field(
            "Summary",
            FieldType::Text {
                max_length: Some(200),
            },
        );
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
            }
        }

        self.add_field(
            "Severity",
            FieldType::Choice {
                options: vec![
                    "Critical".to_string(),
                    "High".to_string(),
                    "Medium".to_string(),
                    "Low".to_string(),
                ],
                multi: false,
            },
        );
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
                field.default = Some("Medium".to_string());
            }
        }

        self.add_field(
            "Component",
            FieldType::Choice {
                options: vec![
                    "UI".to_string(),
                    "Backend".to_string(),
                    "API".to_string(),
                    "Database".to_string(),
                    "Other".to_string(),
                ],
                multi: true,
            },
        );

        self.add_field("Steps to Reproduce", FieldType::TextArea { rows: 6 });
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
            }
        }

        self.add_field("Expected Behavior", FieldType::TextArea { rows: 3 });

        self.add_field("Actual Behavior", FieldType::TextArea { rows: 3 });
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
            }
        }

        self.add_field(
            "Reporter Email",
            FieldType::Text {
                max_length: Some(255),
            },
        );
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.pattern = Some("email".to_string());
            }
        }
    }

    pub fn create_survey_form(&mut self) {
        self.create_form("Feedback Survey");

        self.add_field(
            "Overall Satisfaction",
            FieldType::Choice {
                options: vec![
                    "Very Satisfied".to_string(),
                    "Satisfied".to_string(),
                    "Neutral".to_string(),
                    "Dissatisfied".to_string(),
                    "Very Dissatisfied".to_string(),
                ],
                multi: false,
            },
        );
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
            }
        }

        self.add_field(
            "What did you like most?",
            FieldType::Choice {
                options: vec![
                    "Ease of use".to_string(),
                    "Features".to_string(),
                    "Performance".to_string(),
                    "Design".to_string(),
                    "Support".to_string(),
                ],
                multi: true,
            },
        );

        self.add_field(
            "How likely to recommend? (0-10)",
            FieldType::Number {
                min: Some(0.0),
                max: Some(10.0),
                decimals: 0,
            },
        );
        if let Some(form) = self.current_form_mut() {
            if let Some(field) = form.fields.last_mut() {
                field.validation.required = true;
            }
        }

        self.add_field("Additional Comments", FieldType::TextArea { rows: 4 });

        self.add_field("Contact me about feedback", FieldType::Checkbox);
    }
}
