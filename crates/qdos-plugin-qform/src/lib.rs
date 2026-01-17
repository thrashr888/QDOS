//! Q-FORM: Form Builder Plugin for R-DOS
//!
//! A form builder and data entry tool for creating and filling out forms.
//! Inspired by Microsoft Access forms, Google Forms, and Filemaker.

mod modal;
mod state;

pub use state::{
    DesignerMode, ExportFormat, Field, FieldType, Form, QFormState, QFormView, Record,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, ThemeColors,
};
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::fs;
use std::path::PathBuf;

// =============================================================================
// PLUGIN
// =============================================================================

pub struct QFormPlugin {
    state: QFormState,
}

impl Default for QFormPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QFormPlugin {
    pub fn new() -> Self {
        let mut plugin = Self {
            state: QFormState::new(),
        };
        plugin.load_forms();
        plugin
    }

    // =========================================================================
    // PERSISTENCE
    // =========================================================================

    fn ensure_dirs(&self) {
        if let Some(parent) = self.state.forms_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::create_dir_all(&self.state.records_path);
    }

    fn load_forms(&mut self) {
        self.ensure_dirs();

        if self.state.forms_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.state.forms_path) {
                if let Ok(forms) = serde_json::from_str(&content) {
                    self.state.forms = forms;
                }
            }
        }

        // Load records for all forms
        self.load_all_records();
    }

    fn load_all_records(&mut self) {
        self.state.records.clear();

        if let Ok(entries) = fs::read_dir(&self.state.records_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(records) = serde_json::from_str::<Vec<Record>>(&content) {
                            self.state.records.extend(records);
                        }
                    }
                }
            }
        }
    }

    fn save_forms(&self) -> Result<(), String> {
        self.ensure_dirs();

        let content = serde_json::to_string_pretty(&self.state.forms).map_err(|e| e.to_string())?;
        fs::write(&self.state.forms_path, content).map_err(|e| e.to_string())?;

        Ok(())
    }

    fn save_records(&self, form_id: &str) -> Result<(), String> {
        self.ensure_dirs();

        let form_records: Vec<&Record> = self
            .state
            .records
            .iter()
            .filter(|r| r.form_id == form_id)
            .collect();

        let path = self.state.records_path.join(format!("{}.json", form_id));
        let content = serde_json::to_string_pretty(&form_records).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())?;

        Ok(())
    }

    // =========================================================================
    // EXPORT
    // =========================================================================

    fn export_csv(&self, path: &str) -> Result<String, String> {
        let form = self.state.current_form().ok_or("No form selected")?;
        let records = self.state.records_for_current_form();
        let record_count = records.len();

        let mut wtr = csv::Writer::from_path(path).map_err(|e| e.to_string())?;

        // Header row
        let headers: Vec<&str> = form.fields.iter().map(|f| f.label.as_str()).collect();
        wtr.write_record(&headers).map_err(|e| e.to_string())?;

        // Data rows
        for record in records {
            let row: Vec<String> = form
                .fields
                .iter()
                .map(|f| record.data.get(&f.id).cloned().unwrap_or_default())
                .collect();
            wtr.write_record(&row).map_err(|e| e.to_string())?;
        }

        wtr.flush().map_err(|e| e.to_string())?;

        Ok(format!("Exported {} records to {}", record_count, path))
    }

    fn export_json(&self, path: &str) -> Result<String, String> {
        let records = self.state.records_for_current_form();

        let content = serde_json::to_string_pretty(&records).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())?;

        Ok(format!("Exported {} records to {}", records.len(), path))
    }

    // =========================================================================
    // KEY HANDLERS
    // =========================================================================

    fn handle_form_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => KeyHandleResult::CloseModal,
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.form_cursor_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.form_cursor_down();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.forms.is_empty() {
                    self.state.select_form(self.state.form_cursor);
                    self.state.view = QFormView::Designer;
                    self.state.designer_field = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.state.create_form("New Form");
                self.state.view = QFormView::Designer;
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                // Template menu - create contact form for now
                self.state.create_contact_form();
                self.state.form_cursor = self.state.forms.len().saturating_sub(1);
                let _ = self.save_forms();
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if !self.state.forms.is_empty() {
                    self.state.select_form(self.state.form_cursor);
                    self.state.view = QFormView::Designer;
                    self.state.designer_field = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if !self.state.forms.is_empty() {
                    self.state.select_form(self.state.form_cursor);
                    self.state.start_entry();
                    self.state.view = QFormView::Entry;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if !self.state.forms.is_empty() {
                    self.state.select_form(self.state.form_cursor);
                    self.state.view = QFormView::Records;
                    self.state.record_cursor = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                if !self.state.forms.is_empty() {
                    self.state.select_form(self.state.form_cursor);
                    self.state.view = QFormView::Export;
                    self.state.export_path = "export.csv".to_string();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Delete | KeyCode::Backspace => {
                if !self.state.forms.is_empty() {
                    self.state.delete_form(self.state.form_cursor);
                    let _ = self.save_forms();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('?') => {
                self.state.view = QFormView::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_designer_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.state.designer_mode {
            DesignerMode::Navigate => self.handle_designer_navigate_key(key),
            DesignerMode::EditLabel => self.handle_designer_edit_key(key),
            DesignerMode::EditType => self.handle_designer_type_key(key),
            DesignerMode::AddField => self.handle_designer_add_key(key),
            DesignerMode::EditOptions => self.handle_designer_options_key(key),
        }
    }

    fn handle_designer_navigate_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QFormView::FormList;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.state.move_field_up();
                } else {
                    self.state.designer_cursor_up();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.state.move_field_down();
                } else {
                    self.state.designer_cursor_down();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.state.designer_mode = DesignerMode::AddField;
                self.state.field_edit_buffer.clear();
                KeyHandleResult::Handled
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if let Some(form) = self.state.current_form() {
                    if let Some(field) = form.fields.get(self.state.designer_field) {
                        self.state.field_edit_buffer = field.label.clone();
                        self.state.designer_mode = DesignerMode::EditLabel;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.state.type_cursor = 0;
                self.state.designer_mode = DesignerMode::EditType;
                KeyHandleResult::Handled
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                // Toggle required
                let designer_field = self.state.designer_field;
                if let Some(form_idx) = self.state.current_form {
                    if let Some(field) = self.state.forms[form_idx].fields.get_mut(designer_field) {
                        field.validation.required = !field.validation.required;
                        self.state.modified = true;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.state.remove_field(self.state.designer_field);
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                match self.save_forms() {
                    Ok(()) => {
                        self.state.modified = false;
                        self.state.status_message = Some("Form saved".to_string());
                    }
                    Err(e) => {
                        self.state.status_message = Some(format!("Save failed: {}", e));
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('?') => {
                self.state.view = QFormView::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_designer_edit_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.designer_mode = DesignerMode::Navigate;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let designer_field = self.state.designer_field;
                let new_label = self.state.field_edit_buffer.clone();
                if let Some(form_idx) = self.state.current_form {
                    if let Some(field) = self.state.forms[form_idx].fields.get_mut(designer_field) {
                        field.label = new_label;
                        self.state.modified = true;
                    }
                }
                self.state.designer_mode = DesignerMode::Navigate;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.field_edit_buffer.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.field_edit_buffer.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_designer_type_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let types = FieldType::all_types();
        match key.code {
            KeyCode::Esc => {
                self.state.designer_mode = DesignerMode::Navigate;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.type_cursor > 0 {
                    self.state.type_cursor -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.type_cursor + 1 < types.len() {
                    self.state.type_cursor += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let type_cursor = self.state.type_cursor;
                let designer_field = self.state.designer_field;
                if let Some(new_type) = types.get(type_cursor) {
                    if let Some(form_idx) = self.state.current_form {
                        if let Some(field) =
                            self.state.forms[form_idx].fields.get_mut(designer_field)
                        {
                            field.field_type = new_type.clone();
                            self.state.modified = true;
                        }
                    }
                }
                self.state.designer_mode = DesignerMode::Navigate;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_designer_add_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.designer_mode = DesignerMode::Navigate;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.field_edit_buffer.is_empty() {
                    self.state.add_field(
                        &self.state.field_edit_buffer.clone(),
                        FieldType::Text { max_length: None },
                    );
                    self.state.field_edit_buffer.clear();
                }
                self.state.designer_mode = DesignerMode::Navigate;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.field_edit_buffer.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.field_edit_buffer.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_designer_options_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.state.designer_mode = DesignerMode::Navigate;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_entry_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QFormView::FormList;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.state.entry_prev_field();
                } else {
                    self.state.entry_next_field();
                }
                self.state.choice_cursor = 0;
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                self.state.entry_prev_field();
                self.state.choice_cursor = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                // For choice fields, move cursor up
                if let Some(form) = self.state.current_form() {
                    if let Some(field) = form.fields.get(self.state.entry_field) {
                        if let FieldType::Choice { options, .. } = &field.field_type {
                            if self.state.choice_cursor > 0 {
                                self.state.choice_cursor -= 1;
                            } else if options.is_empty() {
                                self.state.entry_prev_field();
                            }
                        } else {
                            self.state.entry_prev_field();
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                // For choice fields, move cursor down
                if let Some(form) = self.state.current_form() {
                    if let Some(field) = form.fields.get(self.state.entry_field) {
                        if let FieldType::Choice { options, .. } = &field.field_type {
                            if self.state.choice_cursor + 1 < options.len() {
                                self.state.choice_cursor += 1;
                            } else {
                                self.state.entry_next_field();
                            }
                        } else {
                            self.state.entry_next_field();
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Submit
                    match self.state.submit_entry() {
                        Ok(()) => {
                            if let Some(form) = self.state.current_form() {
                                let _ = self.save_records(&form.id);
                            }
                            self.state.view = QFormView::FormList;
                            return KeyHandleResult::CloseWithSuccess(
                                "Record submitted".to_string(),
                            );
                        }
                        Err(e) => {
                            self.state.status_message = Some(e);
                        }
                    }
                } else {
                    // Toggle checkbox or select choice
                    self.handle_entry_toggle();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(' ') => {
                self.handle_entry_toggle();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                let mut value = self.state.current_entry_value();
                value.pop();
                self.state.set_current_entry_value(value);
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                // Only allow typing in text fields
                if let Some(form) = self.state.current_form() {
                    if let Some(field) = form.fields.get(self.state.entry_field) {
                        match &field.field_type {
                            FieldType::Text { max_length } => {
                                let mut value = self.state.current_entry_value();
                                if max_length.map(|m| value.len() < m).unwrap_or(true) {
                                    value.push(c);
                                    self.state.set_current_entry_value(value);
                                }
                            }
                            FieldType::Number { .. } => {
                                if c.is_ascii_digit() || c == '.' || c == '-' {
                                    let mut value = self.state.current_entry_value();
                                    value.push(c);
                                    self.state.set_current_entry_value(value);
                                }
                            }
                            FieldType::Date { .. } => {
                                if c.is_ascii_digit() || c == '-' || c == '/' {
                                    let mut value = self.state.current_entry_value();
                                    value.push(c);
                                    self.state.set_current_entry_value(value);
                                }
                            }
                            FieldType::TextArea { .. } => {
                                let mut value = self.state.current_entry_value();
                                value.push(c);
                                self.state.set_current_entry_value(value);
                            }
                            _ => {}
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_entry_toggle(&mut self) {
        if let Some(form) = self.state.current_form() {
            if let Some(field) = form.fields.get(self.state.entry_field) {
                match &field.field_type {
                    FieldType::Checkbox => {
                        let value = self.state.current_entry_value();
                        let new_value = if value == "true" { "false" } else { "true" };
                        self.state.set_current_entry_value(new_value.to_string());
                    }
                    FieldType::Choice { options, multi } => {
                        if let Some(selected_option) = options.get(self.state.choice_cursor) {
                            if *multi {
                                // Toggle in multi-select
                                let current = self.state.current_entry_value();
                                let mut selected: Vec<&str> = current
                                    .split(',')
                                    .map(|s| s.trim())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                if selected.contains(&selected_option.as_str()) {
                                    selected.retain(|s| *s != selected_option);
                                } else {
                                    selected.push(selected_option);
                                }
                                self.state.set_current_entry_value(selected.join(", "));
                            } else {
                                // Single select
                                self.state.set_current_entry_value(selected_option.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_records_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QFormView::FormList;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.record_cursor_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.record_cursor_down();
                KeyHandleResult::Handled
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.state.view = QFormView::Export;
                self.state.export_path = "export.csv".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Delete | KeyCode::Backspace => {
                // Delete selected record
                let records = self.state.records_for_current_form();
                if let Some(record) = records.get(self.state.record_cursor) {
                    let record_id = record.id.clone();
                    self.state.records.retain(|r| r.id != record_id);
                    if let Some(form) = self.state.current_form() {
                        let _ = self.save_records(&form.id);
                    }
                    if self.state.record_cursor > 0 {
                        self.state.record_cursor -= 1;
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_export_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QFormView::Records;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                self.state.export_format = match self.state.export_format {
                    ExportFormat::Csv => ExportFormat::Json,
                    ExportFormat::Json => ExportFormat::Csv,
                };
                // Update extension
                let ext = match self.state.export_format {
                    ExportFormat::Csv => "csv",
                    ExportFormat::Json => "json",
                };
                if let Some(base) = self.state.export_path.rsplit_once('.') {
                    self.state.export_path = format!("{}.{}", base.0, ext);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let result = match self.state.export_format {
                    ExportFormat::Csv => self.export_csv(&self.state.export_path),
                    ExportFormat::Json => self.export_json(&self.state.export_path),
                };
                match result {
                    Ok(msg) => {
                        self.state.view = QFormView::Records;
                        return KeyHandleResult::CloseWithSuccess(msg);
                    }
                    Err(e) => {
                        self.state.status_message = Some(e);
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.export_path.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.export_path.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QFormView::FormList;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

// =============================================================================
// PLUGIN TRAIT
// =============================================================================

impl Plugin for QFormPlugin {
    fn id(&self) -> &str {
        "qform"
    }

    fn name(&self) -> &str {
        "Q-FORM"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Q-FORM".to_string(),
            description: "Form builder and data entry".to_string(),
            category: PluginCategory::Tools,
            key: 'F',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state.view = QFormView::FormList;
        self.state.form_cursor = 0;
        self.load_forms();
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Q-FORM is launched via Apps menu (F12) which calls launch()
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QFormView::FormList => self.handle_form_list_key(key),
            QFormView::Designer => self.handle_designer_key(key),
            QFormView::Entry => self.handle_entry_key(key),
            QFormView::Records => self.handle_records_key(key),
            QFormView::Export => self.handle_export_key(key),
            QFormView::Help => self.handle_help_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_qform(&self.state, frame, area, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-FORM - Form Builder & Data Entry".to_string(),
            "".to_string(),
            "Create forms with various field types and collect data.".to_string(),
            "".to_string(),
            "Form List:".to_string(),
            "  N        Create new form".to_string(),
            "  T        Create from template".to_string(),
            "  Enter    Open selected form".to_string(),
            "  D        Open in designer".to_string(),
            "  E        Open in entry mode".to_string(),
            "  R        View records".to_string(),
            "  X        Export records".to_string(),
            "".to_string(),
            "Designer:".to_string(),
            "  A        Add field".to_string(),
            "  E        Edit label".to_string(),
            "  T        Change type".to_string(),
            "  V        Toggle required".to_string(),
            "  Ctrl+S   Save".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
