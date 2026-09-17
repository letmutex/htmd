use crate::{
    Context, Element,
    element_handler::element_util::serialize_if_extra_attrs_or_inline,
    element_handler::{HandlerResult, Handlers},
    html_block::starts_html_block,
    options::HeadingStyle,
    util::text::TrimDocumentWhitespace,
};

pub(super) fn headings_handler(
    handlers: &dyn Handlers,
    element: &Element,
) -> Option<HandlerResult> {
    serialize_if_extra_attrs_or_inline!(handlers, element, 0);
    let level = element.tag.chars().nth(1).unwrap() as u32 - '0' as u32;
    // A heading is a leaf block: its children begin an inline context.
    let content = handlers.walk_children_content(element.node, Context::INLINE);
    let content = content.trim_document_whitespace();
    let content = content.trim_matches('\n');

    // A setext heading leaves its content at the start of a line, where an HTML
    // block opening would dissolve the heading; an ATX heading's `#` is leaf
    // block syntax the block scan matches first, so it is safe. Empty content
    // has no setext spelling at all -- its underline is empty too, leaving
    // nothing behind -- so it falls back to ATX as well. See the "Special case
    // for paragraphs" and "Headings" sections of `unsupported_html.md`.
    let use_setext = (level == 1 || level == 2)
        && handlers.options().heading_style == HeadingStyle::Setex
        && !content.is_empty()
        && !starts_html_block(content);

    let mut result = String::from("\n\n");
    if use_setext {
        result.push_str(content);
        result.push('\n');
        let ch = if level == 1 { "=" } else { "-" };
        result.push_str(&ch.repeat(content.chars().count()));
        result.push_str("\n\n");
    } else {
        result.push_str(&"#".repeat(level as usize));
        // An empty heading is just the `#`s: a space with nothing after it
        // would leave trailing whitespace on the line.
        if !content.is_empty() {
            result.push(' ');
            result.push_str(content);
        }
        result.push_str("\n\n");
    }
    Some(result.into())
}
