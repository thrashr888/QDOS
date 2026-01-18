//! Q-DOCS Export Operations
//!
//! Handles conversion of documents to HTML, PDF, and other formats.

use std::path::Path;
use std::process::Command;

/// Convert markdown content to HTML
pub fn markdown_to_html(lines: &[String], title: &str) -> String {
    let mut html = String::new();

    // HTML header
    html.push_str(&format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>{}</title>
<style>
body {{ font-family: "Times New Roman", serif; max-width: 800px; margin: 40px auto; padding: 20px; }}
h1, h2, h3, h4, h5, h6 {{ color: #333; }}
code {{ background: #f4f4f4; padding: 2px 6px; }}
pre {{ background: #f4f4f4; padding: 10px; overflow-x: auto; }}
blockquote {{ border-left: 4px solid #ccc; margin: 0; padding-left: 16px; color: #666; }}
hr {{ border: none; border-top: 1px solid #ccc; }}
</style>
</head>
<body>
"#,
        html_escape(title)
    ));

    let mut in_code_block = false;
    let mut in_list = false;

    for line in lines {
        let trimmed = line.trim();

        // Code blocks
        if trimmed.starts_with("```") {
            if in_code_block {
                html.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                html.push_str("<pre><code>");
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            html.push_str(&html_escape(line));
            html.push('\n');
            continue;
        }

        // Headings
        if let Some(rest) = trimmed.strip_prefix("### ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h3>{}</h3>\n", convert_inline_markdown(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h2>{}</h2>\n", convert_inline_markdown(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h1>{}</h1>\n", convert_inline_markdown(rest)));
        }
        // Lists
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            html.push_str(&format!(
                "<li>{}</li>\n",
                convert_inline_markdown(&trimmed[2..])
            ));
        }
        // Blockquotes
        else if let Some(rest) = trimmed.strip_prefix("> ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!(
                "<blockquote>{}</blockquote>\n",
                convert_inline_markdown(rest)
            ));
        }
        // Horizontal rules
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str("<hr>\n");
        }
        // Empty lines
        else if trimmed.is_empty() {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
        }
        // Regular paragraphs
        else {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<p>{}</p>\n", convert_inline_markdown(trimmed)));
        }
    }

    if in_list {
        html.push_str("</ul>\n");
    }

    html.push_str("</body>\n</html>");
    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn convert_inline_markdown(text: &str) -> String {
    let escaped = html_escape(text);
    let mut result = escaped;

    // Bold (**text**)
    while let Some(start) = result.find("**") {
        if let Some(end) = result[start + 2..].find("**") {
            let before = &result[..start];
            let middle = &result[start + 2..start + 2 + end];
            let after = &result[start + 2 + end + 2..];
            result = format!("{}<strong>{}</strong>{}", before, middle, after);
        } else {
            break;
        }
    }

    // Italic (*text*) - be careful not to match ** which is already processed
    let chars: Vec<char> = result.chars().collect();
    let mut i = 0;
    let mut new_result = String::new();

    while i < chars.len() {
        if chars[i] == '*' && (i == 0 || chars[i - 1] != '*') {
            // Find closing *
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '*') {
                if i + 1 + end < chars.len() && chars[i + 1 + end + 1] != '*' {
                    // Valid single italic
                    new_result.push_str("<em>");
                    for ch in &chars[i + 1..i + 1 + end] {
                        new_result.push(*ch);
                    }
                    new_result.push_str("</em>");
                    i = i + 1 + end + 1;
                    continue;
                }
            }
        }
        new_result.push(chars[i]);
        i += 1;
    }
    result = new_result;

    // Inline code (`text`)
    while let Some(start) = result.find('`') {
        if let Some(end) = result[start + 1..].find('`') {
            let before = &result[..start];
            let middle = &result[start + 1..start + 1 + end];
            let after = &result[start + 1 + end + 1..];
            result = format!("{}<code>{}</code>{}", before, middle, after);
        } else {
            break;
        }
    }

    // Links [text](url)
    while let Some(bracket_start) = result.find('[') {
        if let Some(bracket_end) = result[bracket_start..].find("](") {
            let bracket_end = bracket_start + bracket_end;
            if let Some(paren_end) = result[bracket_end + 2..].find(')') {
                let paren_end = bracket_end + 2 + paren_end;
                let text = &result[bracket_start + 1..bracket_end];
                let url = &result[bracket_end + 2..paren_end];
                let before = &result[..bracket_start];
                let after = &result[paren_end + 1..];
                result = format!("{}<a href=\"{}\">{}</a>{}", before, url, text, after);
                continue;
            }
        }
        break;
    }

    result
}

/// Export document to HTML file
pub fn export_html(lines: &[String], title: &str, output_path: &Path) -> Result<(), String> {
    let html = markdown_to_html(lines, title);
    std::fs::write(output_path, html).map_err(|e| format!("Failed to write HTML: {}", e))
}

/// Export document to plain text file (no markdown formatting)
pub fn export_plain_text(lines: &[String], output_path: &Path) -> Result<(), String> {
    let content = lines.join("\n");
    std::fs::write(output_path, content).map_err(|e| format!("Failed to write text: {}", e))
}

/// Check if pandoc is available
pub fn pandoc_available() -> bool {
    Command::new("pandoc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Export document to PDF via pandoc
pub fn export_pdf(lines: &[String], output_path: &Path, cwd: &Path) -> Result<(), String> {
    if !pandoc_available() {
        return Err("pandoc not found. Install pandoc to export PDF.".to_string());
    }

    // Write markdown to temp file
    let temp_md = cwd.join(".qdocs_export_temp.md");
    let content = lines.join("\n");
    std::fs::write(&temp_md, &content).map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Run pandoc
    let result = Command::new("pandoc")
        .args([
            temp_md.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .current_dir(cwd)
        .output();

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_md);

    match result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).to_string()),
        Err(e) => Err(format!("Failed to run pandoc: {}", e)),
    }
}
