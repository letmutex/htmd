pub(crate) trait TrimAsciiWhitespace {
    fn trim_ascii_whitespace(&self) -> &str;

    fn trim_start_ascii_whitespace(&self) -> &str;

    fn trim_end_ascii_whitespace(&self) -> &str;
}

impl<S> TrimAsciiWhitespace for S
where
    S: AsRef<str>,
{
    #[inline]
    fn trim_ascii_whitespace(&self) -> &str {
        self.as_ref()
            .trim_matches(|ch: char| ch.is_ascii_whitespace())
    }

    #[inline]
    fn trim_start_ascii_whitespace(&self) -> &str {
        self.as_ref()
            .trim_start_matches(|ch: char| ch.is_ascii_whitespace())
    }

    #[inline]
    fn trim_end_ascii_whitespace(&self) -> &str {
        self.as_ref()
            .trim_end_matches(|ch: char| ch.is_ascii_whitespace())
    }
}

pub(crate) trait StripWhitespace {
    /// Strip leading whitespace.
    ///
    /// A tuple of (striped_text, Option<leading_whitespace>) will be returned.
    fn strip_leading_whitespace(&self) -> (&str, Option<&str>);

    /// Strip trailing whitespace.
    ///
    /// A tuple of (striped_text, Option<trailing_whitespace>) will be returned.
    fn strip_trailing_whitespace(&self) -> (&str, Option<&str>);
}

impl<S> StripWhitespace for S
where
    S: AsRef<str>,
{
    fn strip_leading_whitespace(&self) -> (&str, Option<&str>) {
        let text = self.as_ref();
        let mut start = 0;
        for (idx, ch) in text.char_indices() {
            if ch.is_whitespace() {
                start = idx + ch.len_utf8();
            } else {
                break;
            }
        }
        if start != 0 {
            (&text[start..], Some(&text[..start]))
        } else {
            (text, None)
        }
    }

    fn strip_trailing_whitespace(&self) -> (&str, Option<&str>) {
        let text = self.as_ref();
        let mut end: Option<usize> = None;
        for (idx, ch) in text.char_indices().rev() {
            if ch.is_whitespace() {
                end = Some(idx);
            } else {
                break;
            }
        }
        if let Some(end) = end {
            (&text[..end], Some(&text[end..]))
        } else {
            (text, None)
        }
    }
}

pub(crate) fn compress_whitespace(input: &str) -> String {
    let mut result = String::new();
    if input.len() == 0 {
        return result;
    }
    let mut in_whitespace = false;

    for c in input.chars() {
        if c.is_ascii_whitespace() {
            if !in_whitespace {
                result.push(' ');
                in_whitespace = true;
            }
        } else {
            result.push(c);
            in_whitespace = false;
        }
    }

    result
}

pub(crate) fn indent_text_except_first_line(
    text: &str,
    indent: usize,
    trim_line_end: bool,
) -> String {
    if indent == 0 {
        return text.to_string();
    }
    let mut result_lines: Vec<String> = Vec::new();
    let indent_text = " ".repeat(indent);
    for (idx, line) in text.lines().enumerate() {
        let line = if trim_line_end { line.trim_end() } else { line };
        if idx == 0 {
            result_lines.push(line.to_string());
        } else {
            result_lines.push(format!("{}{}", indent_text, line));
        }
    }
    result_lines.join("\n")
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
