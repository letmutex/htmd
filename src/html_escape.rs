use std::borrow::Cow;

/// Escapes HTML-like patterns that would be interpreted as valid HTML by CommonMark
pub(crate) fn escape_html(text: Cow<'_, str>) -> Cow<'_, str> {
    let mut result = String::new();
    let mut chars = text.char_indices().peekable();
    let mut modified = false;

    while let Some((i, ch)) = chars.next() {
        if ch == '<' {
            if let Some(pattern_len) = find_html_pattern(&text[i..]) {
                let pattern = &text[i..i + pattern_len];
                
                // For all patterns, add backslash and then the pattern
                result.push('\\');
                result.push_str(pattern);
                
                modified = true;
                
                // Skip the rest of the pattern by advancing the iterator
                // We need to skip all characters that are within the pattern
                let pattern_end_byte = i + pattern_len;
                while let Some(&(next_i, _)) = chars.peek() {
                    if next_i >= pattern_end_byte {
                        break;
                    }
                    chars.next();
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    if modified {
        Cow::Owned(result)
    } else {
        text
    }
}

/// Check if there's an HTML pattern starting at the given position
/// Returns the length of the pattern if found
fn find_html_pattern(text: &str) -> Option<usize> {
    if !text.starts_with('<') {
        return None;
    }

    // HTML comments: <!-- ... -->
    if text.starts_with("<!--") {
        if let Some(pos) = text.find("-->") {
            return Some(pos + 3);
        }
        return None;
    }

    // Processing instructions: <?...?>
    if text.starts_with("<?") {
        if let Some(pos) = text.find("?>") {
            return Some(pos + 2);
        }
        return None;
    }

    // CDATA: <![CDATA[...]]> - Let markdown escaper handle the brackets
    // Also handle CDATA that has already been markdown-escaped: <!\[CDATA\[...\]\]>
    if text.starts_with("<![CDATA[") || text.starts_with("<!\\[CDATA\\[") {
        return None;
    }

    // Declarations: <!...> (but not comments or CDATA)
    if text.starts_with("<!") && !text.starts_with("<!--") && !text.starts_with("<![CDATA[") && !text.starts_with("<!\\[CDATA\\[") {
        if let Some(pos) = text.find('>') {
            return Some(pos + 1);
        }
        return None;
    }

    // Check for HTML block patterns (case insensitive, beginning of line)
    if is_html_block_pattern(text) {
        return find_end_of_incomplete_tag(text);
    }

    // Regular HTML tags: <tag...> or </tag>
    parse_html_tag(text)
}

/// Check if this looks like an HTML block pattern that should be escaped
fn is_html_block_pattern(text: &str) -> bool {
    // Convert to lowercase for case-insensitive matching
    let lower = text.to_lowercase();
    
    // HTML block type 1: script, pre, textarea, style
    if lower.starts_with("<script") || lower.starts_with("<pre") || 
       lower.starts_with("<textarea") || lower.starts_with("<style") {
        return true;
    }
    
    // HTML block type 6: common block elements
    let block_elements = [
        "address", "article", "aside", "base", "basefont", "blockquote", "body",
        "caption", "center", "col", "colgroup", "dd", "details", "dialog", "dir",
        "div", "dl", "dt", "fieldset", "figcaption", "figure", "footer", "form",
        "frame", "frameset", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header",
        "hr", "html", "iframe", "legend", "li", "link", "main", "menu", "menuitem",
        "nav", "noframes", "ol", "optgroup", "option", "p", "param", "section",
        "source", "summary", "table", "tbody", "td", "tfoot", "th", "thead",
        "title", "tr", "track", "ul"
    ];
    
    for element in &block_elements {
        if lower.starts_with(&format!("<{}", element)) || lower.starts_with(&format!("</{}", element)) {
            return true;
        }
    }
    
    false
}

/// Find the end of an incomplete tag (for block patterns) - returns byte position
fn find_end_of_incomplete_tag(text: &str) -> Option<usize> {
    // For incomplete tags, we want to escape just the opening part
    // Look for space, > or end of string
    for (byte_pos, ch) in text.char_indices().skip(1) {
        if ch.is_ascii_whitespace() || ch == '>' {
            return Some(byte_pos);
        }
    }
    // If no space or >, escape the whole thing
    Some(text.len())
}

/// Parse an HTML tag and return its byte length if valid
fn parse_html_tag(text: &str) -> Option<usize> {
    let mut char_indices = text.char_indices();
    
    // Must start with '<'
    let (_, ch) = char_indices.next()?;
    if ch != '<' {
        return None;
    }

    // Optional '/' for closing tags
    if let Some((_, ch)) = char_indices.clone().next()
        && ch == '/' {
            char_indices.next();
        }

    // Tag name must start with a letter
    let (_, first_char) = char_indices.next()?;
    if !first_char.is_ascii_alphabetic() {
        return None;
    }

    // Continue with tag name (letters, digits, hyphens)
    while let Some((_, ch)) = char_indices.clone().next() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            char_indices.next();
        } else {
            break;
        }
    }

    // Skip to end of tag, handling quoted strings
    let mut in_quotes = false;
    let mut quote_char = '\0';

    for (byte_pos, ch) in char_indices {
        if !in_quotes {
            match ch {
                '"' | '\'' => {
                    in_quotes = true;
                    quote_char = ch;
                }
                '>' => return Some(byte_pos + ch.len_utf8()),
                _ => {}
            }
        } else if ch == quote_char {
            in_quotes = false;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tags() {
        assert_eq!(escape_html("hello".into()), "hello");
        assert_eq!(escape_html("<p>".into()), "\\<p>");
        assert_eq!(escape_html("</p>".into()), "\\</p>");
        assert_eq!(escape_html("<div>content</div>".into()), "\\<div>content\\</div>");
    }

    #[test]
    fn test_comments() {
        assert_eq!(escape_html("<!-- comment -->".into()), "\\<!-- comment -->");
        assert_eq!(escape_html("<!---->".into()), "\\<!---->");
        assert_eq!(escape_html("<!--->".into()), "\\<!--->");
    }

    #[test] 
    fn test_processing_instructions() {
        assert_eq!(escape_html("<?xml version=\"1.0\"?>".into()), "\\<?xml version=\"1.0\"?>");
        assert_eq!(escape_html("<?processing instructions?>".into()), "\\<?processing instructions?>");
    }

    #[test]
    fn test_declarations() {
        assert_eq!(escape_html("<!DOCTYPE html>".into()), "\\<!DOCTYPE html>");
        assert_eq!(escape_html("<!A declaration>".into()), "\\<!A declaration>");
    }

    #[test]
    fn test_cdata() {
        // CDATA is not escaped by HTML escaping, only by markdown escaping later
        assert_eq!(escape_html("<![CDATA[character data]]>".into()), "<![CDATA[character data]]>");
    }

    #[test]
    fn test_tags_with_attributes() {
        assert_eq!(escape_html("<a href=\"test\">".into()), "\\<a href=\"test\">");
        assert_eq!(escape_html("<img src='image.jpg' alt=\"test\"/>".into()), "\\<img src='image.jpg' alt=\"test\"/>");
    }

    #[test]
    fn test_non_html() {
        assert_eq!(escape_html("< not html".into()), "< not html");
        assert_eq!(escape_html("<123>".into()), "<123>");
        assert_eq!(escape_html("< >".into()), "< >");
    }

    #[test]
    fn test_actual_cases() {
        // From the real test cases
        let input = "Test <code>tags</code>, <!-- comments -->, <?processing instructions?>, <!A declaration>, and <![CDATA[character data]]>.";
        // Updated expected result to match current implementation
        let expected = r"Test \<code>tags\</code>, \<!-- comments -->, \<?processing instructions?>, \<!A declaration>, and <![CDATA[character data]]>.";
        assert_eq!(escape_html(input.into()), expected);
    }

    #[test]
    fn test_incomplete_block_tags() {
        // Test incomplete HTML block tags
        assert_eq!(escape_html("<pre".into()), "\\<pre");
        assert_eq!(escape_html("<script".into()), "\\<script");
        assert_eq!(escape_html("<style".into()), "\\<style");
        assert_eq!(escape_html("<address".into()), "\\<address");
        assert_eq!(escape_html("<ul".into()), "\\<ul");
    }

    #[test]
    fn test_unicode_and_emoji() {
        // Test cases that would panic due to character/byte index confusion
        assert_eq!(escape_html("<p😀>".into()), "\\<p😀>");
        assert_eq!(escape_html("<div>😀</div>".into()), "\\<div>😀\\</div>");
        assert_eq!(escape_html("<p class='😀'>".into()), "\\<p class='😀'>");
        assert_eq!(escape_html("<script😀>".into()), "\\<script😀>");
    }
}