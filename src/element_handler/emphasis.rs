use crate::{
    Element,
    element_handler::Chain,
    serialize_if_faithful,
    text_util::{StripWhitespace, concat_strings},
};

pub(super) fn emphasis_handler(
    _chain: &dyn Chain,
    element: Element,
    marker: &str,
) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    let content = element.content;
    if content.is_empty() {
        return (None, true);
    }
    let (content, leading_whitespace) = content.strip_leading_whitespace();
    let (content, trailing_whitespace) = content.strip_trailing_whitespace();
    if content.is_empty() {
        return (None, true);
    }
    (
        Some(concat_strings!(
            leading_whitespace.unwrap_or(""),
            marker,
            content,
            marker,
            trailing_whitespace.unwrap_or("")
        )),
        true,
    )
}
