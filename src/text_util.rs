pub(crate) trait TrimDocumentWhitespace {
    fn trim_document_whitespace(&self) -> &str;

    fn trim_start_document_whitespace(&self) -> &str;

    fn trim_end_document_whitespace(&self) -> &str;
}

impl<S> TrimDocumentWhitespace for S
where
    S: AsRef<str>,
{
    #[inline]
    fn trim_document_whitespace(&self) -> &str {
        self.as_ref().trim_matches(is_document_whitespace)
    }

    #[inline]
    fn trim_start_document_whitespace(&self) -> &str {
        self.as_ref().trim_start_matches(is_document_whitespace)
    }

    #[inline]
    fn trim_end_document_whitespace(&self) -> &str {
        self.as_ref().trim_end_matches(is_document_whitespace)
    }
}

pub(crate) trait StripWhitespace {
    /// Strip leading whitespace.
    ///
    /// A tuple of (striped_text, Option<leading_whitespace>) will be returned.
    fn strip_leading_document_whitespace(&self) -> (&str, Option<&str>);
    fn strip_leading_whitespace(&self) -> (&str, Option<&str>);

    /// Strip trailing whitespace.
    ///
    /// A tuple of (striped_text, Option<trailing_whitespace>) will be returned.
    fn strip_trailing_document_whitespace(&self) -> (&str, Option<&str>);
    fn strip_trailing_whitespace(&self) -> (&str, Option<&str>);
}

impl<S> StripWhitespace for S
where
    S: AsRef<str>,
{
    fn strip_leading_whitespace(&self) -> (&str, Option<&str>) {
        let text = self.as_ref();
        let trimmed_text = text.trim_start();
        let stripped_len = text.len() - trimmed_text.len();
        if stripped_len == 0 {
            (text, None)
        } else {
            let start_index = stripped_len;
            (&text[start_index..], Some(&text[..start_index]))
        }
    }

    fn strip_leading_document_whitespace(&self) -> (&str, Option<&str>) {
        let text = self.as_ref();
        let trimmed_text = text.trim_start_document_whitespace();
        let stripped_len = text.len() - trimmed_text.len();
        if stripped_len == 0 {
            (text, None)
        } else {
            let start_index = stripped_len;
            (&text[start_index..], Some(&text[..start_index]))
        }
    }

    fn strip_trailing_whitespace(&self) -> (&str, Option<&str>) {
        let text = self.as_ref();
        let trimmed_text = text.trim_end();
        let stripped_len = text.len() - trimmed_text.len();
        if stripped_len == 0 {
            (text, None)
        } else {
            let end_index = trimmed_text.len();
            (&text[..end_index], Some(&text[end_index..]))
        }
    }

    fn strip_trailing_document_whitespace(&self) -> (&str, Option<&str>) {
        let text = self.as_ref();
        let trimmed_text = text.trim_end_document_whitespace();
        let stripped_len = text.len() - trimmed_text.len();
        if stripped_len == 0 {
            (text, None)
        } else {
            let end_index = trimmed_text.len();
            (&text[..end_index], Some(&text[end_index..]))
        }
    }
}

pub(crate) trait JoinOnStringIterator {
    fn join<S: AsRef<str>>(&mut self, separator: S) -> String;
}

impl<T, S> JoinOnStringIterator for T
where
    S: AsRef<str>,
    T: Iterator<Item = S>,
{
    fn join<SE: AsRef<str>>(&mut self, separator: SE) -> String {
        let Some(first) = self.next() else {
            return String::new();
        };
        let separator = separator.as_ref();
        let mut result = String::from(first.as_ref());
        for next in self {
            result.push_str(separator);
            result.push_str(next.as_ref());
        }
        result
    }
}

/// Join text clips, inspired by:
/// https://github.com/mixmark-io/turndown/blob/cc73387fb707e5fb5e1083e94078d08f38f3abc8/src/turndown.js#L221
pub(crate) fn join_blocks(contents: &[String]) -> String {
    // Pre-allocate capacity to avoid multiple re-allocations.
    let capacity = contents.iter().map(String::len).sum();
    let mut result = String::with_capacity(capacity);

    for content in contents {
        let content_len = content.len();
        if content_len == 0 {
            continue;
        }

        let result_len = result.len();
        let left = result.trim_end_matches('\n');
        let right = content.trim_start_matches('\n');

        let max_trimmed_new_lines =
            std::cmp::max(result_len - left.len(), content_len - right.len());
        let separator_new_lines = std::cmp::min(max_trimmed_new_lines, 2);

        // Remove trailing newlines.
        result.truncate(left.len());

        // Add the calculated separator
        if separator_new_lines == 1 {
            result.push('\n');
        } else if separator_new_lines == 2 {
            result.push_str("\n\n");
        }

        // Append the new, trimmed content
        result.push_str(right);
    }
    result
}

pub(crate) fn compress_whitespace(input: &str) -> Cow<'_, str> {
    if input.is_empty() {
        return Cow::Borrowed(input);
    }

    let mut result: Option<String> = None;
    let mut in_whitespace = false;

    // Use char_indices to get byte indices for slicing the input.
    for (byte_index, c) in input.char_indices() {
        if c.is_ascii_whitespace() {
            if in_whitespace {
                // Consecutive whitespace: skip this character.
                if result.is_none() {
                    // Lazy allocation: First change found. Allocate and copy the prefix.
                    let mut s = String::with_capacity(input.len());
                    s.push_str(&input[..byte_index]);
                    result = Some(s);
                }
            } else {
                // First whitespace in sequence.
                in_whitespace = true;
                if c == ' ' {
                    // Valid single space. If already allocating, append it.
                    if let Some(res) = &mut result {
                        res.push(' ');
                    }
                } else {
                    // Non-space whitespace (e.g., \n): must be changed to ' '.
                    if result.is_none() {
                        // Lazy allocation: First change found. Allocate and copy the prefix.
                        let mut s = String::with_capacity(input.len());
                        s.push_str(&input[..byte_index]);
                        result = Some(s);
                    }
                    result.as_mut().unwrap().push(' ');
                }
            }
        } else {
            // Not whitespace.
            in_whitespace = false;
            // If already allocating, append the character.
            if let Some(res) = &mut result {
                res.push(c);
            }
        }
    }

    // If `result` is None, return Cow::Borrowed (no changes were made).
    match result {
        Some(s) => Cow::Owned(s),
        None => Cow::Borrowed(input),
    }
}

// Per [MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Text/Whitespace),
// document white space characters only include spaces, tabs, line
// feeds, and newlines. Remove only these from the end of a line.
fn is_document_whitespace(c: char) -> bool {
    ['\t', '\n', '\r', ' '].contains(&c)
}

pub(crate) fn indent_text_except_first_line(
    text: &str,
    indent: usize,
    trim_line_end: bool,
) -> String {
    if indent == 0 {
        return text.to_string();
    }
    let line_count = text.lines().count();
    let estimated_capacity = text.len() + (line_count.saturating_sub(1)) * indent;
    let mut result = String::with_capacity(estimated_capacity);
    let indent_text = " ".repeat(indent);
    for (idx, line) in text.lines().enumerate() {
        let line = if trim_line_end {
            line.trim_end_matches(is_document_whitespace)
        } else {
            line
        };
        if idx > 0 {
            result.push('\n');
        }
        if idx == 0 || line.is_empty() {
            result.push_str(line);
        } else {
            result.push_str(&concat_strings!(indent_text, line));
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
    let mut is_prev_ch_dot = false;
    for (index, ch) in text.chars().enumerate() {
        if ch.is_numeric() {
            if is_prev_ch_dot {
                return None;
            }
            is_prev_ch_numeric = true;
        } else if ch == '.' {
            if !is_prev_ch_numeric {
                return None;
            }
            is_prev_ch_dot = true;
        } else if ch == ' ' {
            if is_prev_ch_dot {
                return Some(index - 1);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    None
}

macro_rules! concat_strings {
    ($($x:expr),*) => {{
        let mut len = 0;
        $(
            len += &$x.len();
        )*
        let mut result = String::with_capacity(len);
        $(
            result.push_str(&$x);
        )*
        result
    }};
}
use std::borrow::Cow;

pub(crate) use concat_strings;

#[cfg(test)]
mod tests {
    use super::index_of_markdown_ordered_item_dot;

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
}
