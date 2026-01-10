//! Integration tests for R-DOS TUI

use ratatui::{backend::TestBackend, Terminal};

/// Helper to extract text from the terminal buffer
fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            result.push_str(cell.symbol());
        }
        result.push('\n');
    }
    result
}

#[test]
fn test_app_renders_rdos_title() {
    // Create app in a temp directory
    let temp_dir = std::env::temp_dir();
    let app = rdos::app::App::new(&temp_dir.to_string_lossy()).expect("Failed to create app");

    // Create a test terminal with 80x25 size (classic DOS dimensions)
    let backend = TestBackend::new(80, 25);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    // Render the UI
    terminal
        .draw(|frame| {
            rdos::ui::draw(frame, &app);
        })
        .expect("Failed to draw");

    // Extract buffer content
    let content = buffer_to_string(&terminal);

    // Check for "R-DOS" in the rendered output
    assert!(
        content.contains("R-DOS"),
        "Expected 'R-DOS' in UI output. Got:\n{}",
        content
    );
}

#[test]
fn test_app_minimum_size_renders() {
    let temp_dir = std::env::temp_dir();
    let app = rdos::app::App::new(&temp_dir.to_string_lossy()).expect("Failed to create app");

    // Test with exactly minimum size (80x25)
    let backend = TestBackend::new(80, 25);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    terminal
        .draw(|frame| {
            rdos::ui::draw(frame, &app);
        })
        .expect("Failed to draw");

    let content = buffer_to_string(&terminal);

    // Should NOT show the "too small" message
    assert!(
        !content.contains("Terminal too small"),
        "Should not show 'too small' message at 80x25"
    );
}

#[test]
fn test_app_too_small_shows_message() {
    let temp_dir = std::env::temp_dir();
    let app = rdos::app::App::new(&temp_dir.to_string_lossy()).expect("Failed to create app");

    // Test with terminal smaller than minimum
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    terminal
        .draw(|frame| {
            rdos::ui::draw(frame, &app);
        })
        .expect("Failed to draw");

    let content = buffer_to_string(&terminal);

    // Should show the "too small" message
    assert!(
        content.contains("Terminal too small"),
        "Expected 'Terminal too small' message for undersized terminal"
    );
}
