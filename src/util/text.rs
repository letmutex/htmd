use std::borrow::Cow;

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

pub(crate) use concat_strings;

// Per [MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Text/Whitespace),
// document white space characters only include spaces, tabs, line
// feeds, and newlines. Remove only these from the end of a line.
#[inline]
fn is_document_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | ' ')
}

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

pub(crate) fn append_block(output: &mut String, content: &str) {
    if content.is_empty() {
        return;
    }

    let output_without_newlines = output.trim_end_matches('\n').len();
    let content_without_newlines = content.trim_start_matches('\n');
    let boundary_newlines = (output.len() - output_without_newlines)
        .max(content.len() - content_without_newlines.len())
        .min(2);

    output.truncate(output_without_newlines);
    for _ in 0..boundary_newlines {
        output.push('\n');
    }
    output.push_str(content_without_newlines);
}

/// Frames `content` as a CommonMark block: any newlines already around it are
/// dropped and a blank line is written on either side. The walk collapses a run
/// of newlines at a boundary to two, so blocks framed this way end up separated
/// by exactly one blank line however they are combined.
pub(crate) fn frame_as_block(content: &str) -> String {
    concat_strings!("\n\n", content.trim_matches('\n'), "\n\n")
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
