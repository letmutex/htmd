use std::borrow::Cow;

use super::text::TrimDocumentWhitespace;

/// Escape sequences that Markdown would treat as raw HTML so that they stay as literal text.
pub(crate) fn escape_html(text: Cow<'_, str>) -> Cow<'_, str> {
    let src = text.as_ref();
    if !src.contains('<') {
        return text;
    }

    let mut escaped = String::with_capacity(src.len());
    let mut modified = false;
    let mut last_copy_index = 0;

    for (idx, ch) in src.char_indices() {
        if ch != '<' {
            continue;
        }

        if should_escape_html_like_sequence(&src[idx..]) {
            escaped.push_str(&src[last_copy_index..idx]);
            escaped.push('\\');
            escaped.push('<');
            modified = true;
            last_copy_index = idx + 1;
        }
    }

    if !modified {
        return text;
    }

    escaped.push_str(&src[last_copy_index..]);
    Cow::Owned(escaped)
}

fn should_escape_html_like_sequence(fragment: &str) -> bool {
    let mut chars = fragment.chars();
    let Some('<') = chars.next() else {
        return false;
    };

    let Some(next) = chars.next() else {
        return false;
    };

    match next {
        '!' => {
            let rest = chars.as_str();
            !(rest.starts_with("[CDATA[") || rest.starts_with("\\[CDATA\\["))
        }
        '?' => true,
        '/' => chars.next().is_some_and(|c| c.is_ascii_alphabetic()),
        c if c.is_ascii_alphabetic() => true,
        _ => false,
    }
}

/// Whether `text` holds a line ending: the one place the set of line endings
/// the escapes here and in `element_util` know about is written down.
pub(crate) fn has_line_ending(text: &str) -> bool {
    text.contains(['\r', '\n'])
}

/// Pushes `ch` onto `output`, writing a line ending as the character reference
/// a CommonMark parser decodes back into it. Encoding this way lets a line
/// ending sit inside a leaf block — a raw HTML inline, a link destination, a
/// quoted title — which a bare one would end.
///
/// Being per-character, this is no use where a CRLF pair has to count as the
/// one line ending it is; see `element_util::escape_html_block_blank_lines`.
pub(crate) fn push_encoding_line_ending(output: &mut String, ch: char) {
    match ch {
        '\r' => output.push_str("&#13;"),
        '\n' => output.push_str("&#10;"),
        _ => output.push(ch),
    }
}

/// Escapes a [link destination](https://spec.commonmark.org/0.31.2/#link-destination):
/// the parentheses which would close it early, and the line endings it may not
/// hold at all. A line ending becomes a character reference for the reason
/// [`normalize_title`] gives.
pub(crate) fn escape_link_destination(link: String) -> String {
    if !link.contains(['(', ')']) && !has_line_ending(&link) {
        return link;
    }

    let mut escaped = String::with_capacity(link.len());
    for ch in link.chars() {
        match ch {
            '(' => escaped.push_str("\\("),
            ')' => escaped.push_str("\\)"),
            _ => push_encoding_line_ending(&mut escaped, ch),
        }
    }
    escaped
}

/// Normalizes an `alt` or `title` attribute value for Markdown: each line is
/// trimmed of document whitespace, blank lines are dropped, every `"` is
/// escaped so the value can sit inside a quoted title, and what is left is
/// joined with `&#10;`.
///
/// The join is a character reference rather than the line ending it replaces
/// because the value ends up inside a CommonMark inline — a link destination's
/// title, or the label an image takes its alt text from — and a line ending
/// there ends the leaf block holding it, truncating an ATX heading or a table
/// row. CommonMark recognizes character references in both places, so `&#10;`
/// decodes back to the line ending it replaced.
pub(crate) fn normalize_title(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line.trim_document_whitespace();
        if line.is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push_str("&#10;");
        }
        for ch in line.chars() {
            match ch {
                '"' => result.push_str("\\\""),
                // `lines()` splits on a line feed, so a lone carriage return —
                // which an attribute value can still hold, written as `&#13;` —
                // reaches here intact and needs the same encoding.
                _ => push_encoding_line_ending(&mut result, ch),
            }
        }
    }
    result
}

pub(crate) fn is_markdown_atx_heading(text: &str) -> bool {
    let mut is_prev_ch_hash = false;
    for ch in text.chars() {
        if ch == '#' {
            is_prev_ch_hash = true;
        } else if ch == ' ' {
            return is_prev_ch_hash;
        } else {
            return false;
        }
    }
    false
}

pub(crate) fn index_of_markdown_ordered_item_dot(text: &str) -> Option<usize> {
    let mut is_prev_ch_numeric = false;
    let mut dot_byte_offset = 0;
    let mut is_prev_ch_dot = false;
    for (byte_offset, ch) in text.char_indices() {
        if ch.is_numeric() {
            if is_prev_ch_dot {
                return None;
            }
            is_prev_ch_numeric = true;
        } else if ch == '.' {
            if !is_prev_ch_numeric {
                return None;
            }
            dot_byte_offset = byte_offset;
            is_prev_ch_dot = true;
        } else if ch == ' ' {
            if is_prev_ch_dot {
                return Some(dot_byte_offset);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{escape_html, index_of_markdown_ordered_item_dot};

    #[test]
    fn escapes_basic_tags() {
        assert_eq!(escape_html("<p>".into()), "\\<p>");
        assert_eq!(escape_html("</p>".into()), "\\</p>");
        assert_eq!(
            escape_html("<div>content</div>".into()),
            "\\<div>content\\</div>"
        );
    }

    #[test]
    fn escapes_misc_sequences() {
        assert_eq!(escape_html("<!-- comment -->".into()), "\\<!-- comment -->");
        assert_eq!(
            escape_html("<?xml version=\"1.0\"?>".into()),
            "\\<?xml version=\"1.0\"?>"
        );
        assert_eq!(escape_html("<!DOCTYPE html>".into()), "\\<!DOCTYPE html>");
        assert_eq!(escape_html("<pre".into()), "\\<pre");
    }

    #[test]
    fn leaves_non_html_sequences() {
        assert_eq!(escape_html("< not html".into()), "< not html");
        assert_eq!(escape_html("<123>".into()), "<123>");
        assert_eq!(escape_html("< >".into()), "< >");
    }

    #[test]
    fn leaves_cdata_sections() {
        assert_eq!(
            escape_html("<![CDATA[character data]]>".into()),
            "<![CDATA[character data]]>"
        );
        assert_eq!(
            escape_html("<!\\[CDATA\\[already escaped]]>".into()),
            "<!\\[CDATA\\[already escaped]]>"
        );
    }

    #[test]
    fn test_index_of_markdown_ordered_item_dot() {
        assert_eq!(None, index_of_markdown_ordered_item_dot("16.1¾ "));
        assert_eq!(Some(1), index_of_markdown_ordered_item_dot("1. "));
        assert_eq!(Some(2), index_of_markdown_ordered_item_dot("12. "));
        assert_eq!(Some(5), index_of_markdown_ordered_item_dot("12345. "));
        assert_eq!(Some(1), index_of_markdown_ordered_item_dot("1. \n"));
        assert_eq!(None, index_of_markdown_ordered_item_dot(". "));
        assert_eq!(None, index_of_markdown_ordered_item_dot("abc. "));
        assert_eq!(None, index_of_markdown_ordered_item_dot("1 . "));
        assert_eq!(None, index_of_markdown_ordered_item_dot(" 1. "));
        assert_eq!(None, index_of_markdown_ordered_item_dot("1.a "));
        assert_eq!(None, index_of_markdown_ordered_item_dot("1."));
    }

    #[test]
    fn test_index_of_markdown_ordered_item_dot_multibyte() {
        // U+00BD (½) is 2 bytes in UTF-8: the dot byte offset is 3, not 2
        assert_eq!(Some(3), index_of_markdown_ordered_item_dot("2½. text"));
        // No dot, should return None
        assert_eq!(None, index_of_markdown_ordered_item_dot("2½"));
    }
}
