use crate::{
    Element,
    element_handler::{
        HandlerResult, Handlers,
        element_util::{
            serialize_element, serialize_element_when_faithful, serialize_if_extra_attrs,
        },
    },
    options::TranslationMode,
    util::{
        text::{StripWhitespace, concat_strings},
        unicode::is_unicode_punctuation,
    },
};

pub(super) fn emphasis_handler(
    handlers: &dyn Handlers,
    element: &Element,
    marker: &str,
) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(handlers, element, 0);
    let content = handlers.walk_children_content(element.node, element.context);
    // Note: this is whitespace, NOT document whitespace, per the
    // [Commonmark spec](https://spec.commonmark.org/0.31.2/#emphasis-and-strong-emphasis).
    let (content, leading_whitespace) = content.strip_leading_whitespace();
    let (content, trailing_whitespace) = content.strip_trailing_whitespace();
    // Content which is whitespace, or nothing at all, leaves nothing to
    // emphasize, and CommonMark cannot spell an emphasis empty. See the
    // "Inline elements" section of `unsupported_html.md`.
    serialize_element_when_faithful!(handlers, element, content.is_empty());
    // The element goes, but the whitespace it held still separates the text
    // around it. The trailing run is empty, the leading one having taken all of
    // the content.
    if content.is_empty() {
        return Some(leading_whitespace.unwrap_or("").into());
    }
    // See the [spec](unsupported_html.md) section on Inline elements.
    if handlers.options().translation_mode == TranslationMode::Faithful
        && (content.starts_with(is_unicode_punctuation)
            || content.ends_with(is_unicode_punctuation))
    {
        return Some(HandlerResult::html(serialize_element(handlers, element)));
    }

    let content = concat_strings!(
        leading_whitespace.unwrap_or(""),
        marker,
        content,
        marker,
        trailing_whitespace.unwrap_or("")
    );
    Some(content.into())
}
