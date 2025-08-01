use crate::{
    Element,
    text_util::{JoinOnStringIterator, TrimAsciiWhitespace, concat_strings},
};

pub(super) fn blockquote_handler(element: Element) -> Option<String> {
    let content = element.content.trim_start_matches('\n');
    let content = content
        .trim_end_ascii_whitespace()
        .lines()
        .map(|line| concat_strings!("> ", line))
        .join("\n");
    Some(concat_strings!("\n\n", content, "\n\n"))
}
