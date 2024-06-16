use crate::{text_util::TrimAsciiWhitespace, Element};

pub(super) fn blockquote_handler(element: Element) -> Option<String> {
    let content = element.content.trim_start_matches(|ch| ch == '\n');
    let content = content
        .trim_end_ascii_whitespace()
        .lines()
        .map(|line| format!("> {}", line))
        .collect::<Vec<String>>()
        .join("\n");
    Some(format!("\n\n{}\n\n", content))
}
