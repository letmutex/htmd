use crate::{
    text_util::{concat_strings, TrimAsciiWhitespace},
    Element,
};

pub(super) fn blockquote_handler(element: Element) -> Option<String> {
    let content = element.content.trim_start_matches(|ch| ch == '\n');
    let content = content
        .trim_end_ascii_whitespace()
        .lines()
        .map(|line| concat_strings!("> ", line))
        .collect::<Vec<String>>()
        .join("\n");
    Some(concat_strings!("\n\n", content, "\n\n"))
}
