use crate::{
    Element, serialize_if_faithful,
    text_util::{JoinOnStringIterator, TrimAsciiWhitespace, concat_strings},
};

pub(super) fn blockquote_handler(element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    let content = element.content.trim_start_matches('\n');
    let content = content
        .trim_end_ascii_whitespace()
        .lines()
        .map(|line| concat_strings!("> ", line))
        .join("\n");
    (Some(concat_strings!("\n\n", content, "\n\n")), true)
}
