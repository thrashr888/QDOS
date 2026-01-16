//! HTML to Text Rendering
//!
//! Converts HTML to plain text and ANSI-styled text for terminal display.

use std::collections::VecDeque;

// =============================================================================
// HTML TO TEXT CONVERSION
// =============================================================================

/// Result of HTML to text conversion
#[derive(Debug, Clone, Default)]
pub struct HtmlDocument {
    /// Document title
    pub title: String,
    /// Rendered text lines
    pub lines: Vec<String>,
    /// Links found (text, url, line_number)
    pub links: Vec<(String, String, usize)>,
}

/// Simple HTML to text converter
///
/// Strips HTML tags and converts common elements to plain text.
/// This is a simplified parser - not a full HTML parser.
pub fn html_to_text(html: &str) -> HtmlDocument {
    let mut doc = HtmlDocument::default();
    let mut current_line = String::new();
    let mut in_tag = false;
    let mut tag_name = String::new();
    let mut tag_attrs = String::new();
    let mut skip_content = false;
    let mut list_depth: usize = 0;
    let mut link_text = String::new();
    let mut link_url = String::new();
    let mut in_link = false;
    let mut in_pre = false;
    let mut tag_stack: VecDeque<String> = VecDeque::new();

    // Tags whose content we skip
    let skip_tags = ["script", "style", "noscript", "head", "svg", "nav"];
    // Block-level tags that should start new lines
    let block_tags = [
        "p",
        "div",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "li",
        "br",
        "hr",
        "tr",
        "blockquote",
        "pre",
        "article",
        "section",
        "header",
        "footer",
        "main",
    ];

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '<' {
            in_tag = true;
            tag_name.clear();
            tag_attrs.clear();
            i += 1;
            continue;
        }

        if in_tag {
            if c == '>' {
                in_tag = false;
                let (name, is_closing) = parse_tag_name(&tag_name);
                let name_lower = name.to_lowercase();

                if is_closing {
                    // Handle closing tag
                    if name_lower == "a" && in_link {
                        // End of link
                        in_link = false;
                        let link_num = doc.links.len() + 1;
                        doc.links
                            .push((link_text.clone(), link_url.clone(), doc.lines.len()));
                        current_line.push_str(&format!("[{}]", link_num));
                        link_text.clear();
                        link_url.clear();
                    }

                    if skip_tags.contains(&name_lower.as_str()) {
                        skip_content = false;
                    }

                    if name_lower == "pre" {
                        in_pre = false;
                    }

                    if name_lower == "ul" || name_lower == "ol" {
                        list_depth = list_depth.saturating_sub(1);
                    }

                    if block_tags.contains(&name_lower.as_str()) && !current_line.is_empty() {
                        flush_line(&mut doc.lines, &mut current_line);
                    }

                    tag_stack.pop_back();
                } else {
                    // Handle opening tag
                    tag_stack.push_back(name_lower.clone());

                    if skip_tags.contains(&name_lower.as_str()) {
                        skip_content = true;
                    }

                    // Extract title
                    if name_lower == "title" {
                        // Title tag - capture content
                    }

                    // Handle specific tags
                    match name_lower.as_str() {
                        "br" | "hr" => {
                            flush_line(&mut doc.lines, &mut current_line);
                            if name_lower == "hr" {
                                doc.lines.push("─".repeat(60));
                            }
                        }
                        "p" | "div" | "article" | "section" => {
                            if !current_line.is_empty() {
                                flush_line(&mut doc.lines, &mut current_line);
                                doc.lines.push(String::new()); // Empty line
                            }
                        }
                        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                            flush_line(&mut doc.lines, &mut current_line);
                            doc.lines.push(String::new());
                        }
                        "li" => {
                            flush_line(&mut doc.lines, &mut current_line);
                            let indent = "  ".repeat(list_depth);
                            current_line.push_str(&format!("{}• ", indent));
                        }
                        "ul" | "ol" => {
                            list_depth += 1;
                        }
                        "pre" => {
                            in_pre = true;
                            flush_line(&mut doc.lines, &mut current_line);
                        }
                        "blockquote" => {
                            flush_line(&mut doc.lines, &mut current_line);
                            current_line.push_str("> ");
                        }
                        "a" => {
                            // Start of link
                            in_link = true;
                            link_url = extract_href(&tag_attrs);
                            link_text.clear();
                        }
                        _ => {}
                    }
                }

                i += 1;
                continue;
            }

            // Inside tag - collect name and attributes
            if tag_name.is_empty() && !c.is_whitespace() && c != '/' {
                tag_name.push(c);
            } else if tag_name.is_empty() && c == '/' {
                tag_name.push(c);
            } else if !tag_name.is_empty() && !c.is_whitespace() {
                if tag_attrs.is_empty() && c.is_alphanumeric() {
                    tag_name.push(c);
                } else {
                    tag_attrs.push(c);
                }
            } else if !tag_name.is_empty() {
                tag_attrs.push(c);
            }

            i += 1;
            continue;
        }

        // Regular content
        if !skip_content {
            // Handle entities
            if c == '&' {
                let entity_end = chars[i..].iter().position(|&x| x == ';');
                if let Some(end) = entity_end {
                    let entity: String = chars[i..i + end + 1].iter().collect();
                    let replacement = decode_entity(&entity);
                    if in_link {
                        link_text.push_str(&replacement);
                    }
                    current_line.push_str(&replacement);
                    i += end + 1;
                    continue;
                }
            }

            // Handle whitespace in non-pre mode
            if !in_pre && (c == '\n' || c == '\r') {
                if !current_line.is_empty() && !current_line.ends_with(' ') {
                    current_line.push(' ');
                    if in_link {
                        link_text.push(' ');
                    }
                }
            } else if !in_pre && c.is_whitespace() {
                if !current_line.is_empty() && !current_line.ends_with(' ') {
                    current_line.push(' ');
                    if in_link {
                        link_text.push(' ');
                    }
                }
            } else {
                current_line.push(c);
                if in_link {
                    link_text.push(c);
                }
            }

            // In pre mode, preserve newlines
            if in_pre && c == '\n' {
                flush_line(&mut doc.lines, &mut current_line);
            }
        }

        // Extract title content
        if tag_stack.back().map(|s| s.as_str()) == Some("title") && c != '<' && !in_tag {
            doc.title.push(c);
        }

        i += 1;
    }

    // Flush remaining content
    if !current_line.is_empty() {
        flush_line(&mut doc.lines, &mut current_line);
    }

    // Clean up title
    doc.title = doc.title.trim().to_string();

    // Clean up empty lines at start/end
    while doc.lines.first().map(|s| s.is_empty()).unwrap_or(false) {
        doc.lines.remove(0);
    }
    while doc.lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        doc.lines.pop();
    }

    doc
}

fn flush_line(lines: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        lines.push(trimmed);
    }
    current.clear();
}

fn parse_tag_name(tag: &str) -> (&str, bool) {
    let tag = tag.trim();
    if let Some(rest) = tag.strip_prefix('/') {
        (rest.split_whitespace().next().unwrap_or(rest), true)
    } else {
        (tag.split_whitespace().next().unwrap_or(tag), false)
    }
}

fn extract_href(attrs: &str) -> String {
    // Simple href extraction
    if let Some(start) = attrs.to_lowercase().find("href") {
        let rest = &attrs[start + 4..];
        if let Some(quote_start) = rest.find(['"', '\'']) {
            let quote_char = rest.chars().nth(quote_start).unwrap();
            let after_quote = &rest[quote_start + 1..];
            if let Some(quote_end) = after_quote.find(quote_char) {
                return after_quote[..quote_end].to_string();
            }
        }
    }
    String::new()
}

fn decode_entity(entity: &str) -> String {
    match entity {
        "&amp;" => "&".to_string(),
        "&lt;" => "<".to_string(),
        "&gt;" => ">".to_string(),
        "&quot;" => "\"".to_string(),
        "&apos;" => "'".to_string(),
        "&nbsp;" => " ".to_string(),
        "&mdash;" | "&ndash;" => "—".to_string(),
        "&copy;" => "(c)".to_string(),
        "&reg;" => "(R)".to_string(),
        "&trade;" => "(TM)".to_string(),
        "&hellip;" => "...".to_string(),
        "&lsquo;" | "&rsquo;" => "'".to_string(),
        "&ldquo;" | "&rdquo;" => "\"".to_string(),
        _ => {
            // Try numeric entity
            if entity.starts_with("&#") {
                let num_str = &entity[2..entity.len() - 1];
                if let Some(num_str) = num_str.strip_prefix('x') {
                    // Hex
                    if let Ok(n) = u32::from_str_radix(num_str, 16) {
                        if let Some(c) = char::from_u32(n) {
                            return c.to_string();
                        }
                    }
                } else {
                    // Decimal
                    if let Ok(n) = num_str.parse::<u32>() {
                        if let Some(c) = char::from_u32(n) {
                            return c.to_string();
                        }
                    }
                }
            }
            entity.to_string()
        }
    }
}

// =============================================================================
// TEXT WRAPPING
// =============================================================================

/// Wrap a line of text to fit within max_width characters
fn wrap_line(line: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || line.len() <= max_width {
        return vec![line.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    // Preserve leading whitespace for indentation
    let leading_spaces: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let indent_width = leading_spaces.len();
    let content = line.trim_start();

    for word in content.split_whitespace() {
        let word_len = word.chars().count();

        if current_width == 0 {
            // First word on the line
            if word_len + indent_width > max_width {
                // Word is too long, split it
                let mut chars = word.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(max_width - indent_width).collect();
                    result.push(format!("{}{}", leading_spaces, chunk));
                }
            } else {
                current_line = format!("{}{}", leading_spaces, word);
                current_width = indent_width + word_len;
            }
        } else if current_width + 1 + word_len <= max_width {
            // Word fits on current line
            current_line.push(' ');
            current_line.push_str(word);
            current_width += 1 + word_len;
        } else {
            // Start a new line
            result.push(current_line);
            if word_len + indent_width > max_width {
                // Word is too long, split it
                let mut chars = word.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(max_width - indent_width).collect();
                    result.push(format!("{}{}", leading_spaces, chunk));
                }
                current_line = String::new();
                current_width = 0;
            } else {
                current_line = format!("{}{}", leading_spaces, word);
                current_width = indent_width + word_len;
            }
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}

/// Wrap all lines in an HtmlDocument to fit within max_width
pub fn wrap_document(doc: &mut HtmlDocument, max_width: usize) {
    let mut new_lines = Vec::new();
    let mut new_links = Vec::new();

    for (original_line_idx, line) in doc.lines.iter().enumerate() {
        let wrapped = wrap_line(line, max_width);
        let first_new_idx = new_lines.len();

        new_lines.extend(wrapped);

        // Update link line numbers - links on this original line go to the first wrapped line
        for link in &doc.links {
            if link.2 == original_line_idx {
                new_links.push((link.0.clone(), link.1.clone(), first_new_idx));
            }
        }
    }

    doc.lines = new_lines;
    doc.links = new_links;
}

// =============================================================================
// READER MODE (using dom_smoothie)
// =============================================================================

/// Result of reader mode extraction
#[derive(Debug, Clone, Default)]
pub struct ReaderDocument {
    /// Article title
    pub title: String,
    /// Article author/byline
    pub byline: Option<String>,
    /// Main content lines
    pub content: Vec<String>,
    /// Links within the article content
    pub content_links: Vec<(String, String, usize)>,
    /// Navigation links (from header/footer, not in main content)
    pub nav_links: Vec<(String, String)>,
}

/// Extract readable content from HTML using dom_smoothie
///
/// Returns structured content with article text and separate navigation links.
pub fn extract_reader_content(html: &str, max_width: usize) -> ReaderDocument {
    use dom_smoothie::{Config, Readability};

    let mut result = ReaderDocument::default();

    // Configure for text output
    let config = Config {
        text_mode: dom_smoothie::TextMode::Formatted,
        ..Default::default()
    };

    // Try to extract article content
    if let Ok(mut readability) = Readability::new(html, None, Some(config)) {
        if let Ok(article) = readability.parse() {
            result.title = article.title;
            result.byline = article.byline;

            // Parse the extracted article HTML for content and links
            let article_doc = html_to_text(&article.content);

            // Wrap text
            let mut wrapped_doc = HtmlDocument {
                title: result.title.clone(),
                lines: article_doc.lines,
                links: article_doc.links,
            };
            wrap_document(&mut wrapped_doc, max_width);

            result.content = wrapped_doc.lines;
            result.content_links = wrapped_doc.links;

            // Get ALL links from original page for navigation
            let full_doc = html_to_text(html);
            let article_urls: std::collections::HashSet<_> =
                result.content_links.iter().map(|(_, url, _)| url).collect();

            // Navigation links are those NOT in the article content
            for (text, url, _) in full_doc.links {
                if !article_urls.contains(&url) && !text.is_empty() {
                    result.nav_links.push((text, url));
                }
            }

            return result;
        }
    }

    // Fallback: use simple HTML parser with wrapping
    let mut doc = html_to_text(html);
    wrap_document(&mut doc, max_width);

    result.title = doc.title;
    result.content = doc.lines;
    result.content_links = doc.links;

    result
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_html() {
        let html = "<html><body><p>Hello World</p></body></html>";
        let doc = html_to_text(html);
        assert!(doc.lines.iter().any(|l| l.contains("Hello World")));
    }

    #[test]
    fn test_links() {
        let html = r#"<a href="https://example.com">Example</a>"#;
        let doc = html_to_text(html);
        assert_eq!(doc.links.len(), 1);
        assert_eq!(doc.links[0].1, "https://example.com");
    }

    #[test]
    fn test_relative_links() {
        let html = r#"<a href="newest">Newest</a>"#;
        let doc = html_to_text(html);
        assert_eq!(doc.links.len(), 1);
        assert_eq!(doc.links[0].0, "Newest"); // Link text
        assert_eq!(doc.links[0].1, "newest"); // Link URL (relative)
    }

    #[test]
    fn test_links_with_classes() {
        let html = r#"<a href="/path/page" class="link" id="mylink">Page</a>"#;
        let doc = html_to_text(html);
        assert_eq!(doc.links.len(), 1);
        assert_eq!(doc.links[0].1, "/path/page");
    }

    #[test]
    fn test_entities() {
        let html = "&amp; &lt; &gt; &nbsp;";
        let doc = html_to_text(html);
        assert!(doc.lines.iter().any(|l| l.contains("& < >")));
    }

    #[test]
    fn test_title() {
        let html = "<html><head><title>Test Title</title></head><body>Content</body></html>";
        let doc = html_to_text(html);
        assert_eq!(doc.title, "Test Title");
    }
}
