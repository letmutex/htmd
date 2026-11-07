use crate::{
    Context, Element,
    element_handler::element_util::serialize_if_extra_attrs_or_inline,
    element_handler::{HandlerResult, Handlers},
    html_block::starts_html_block,
    options::HeadingStyle,
    util::text::TrimDocumentWhitespace,
};

/// Translates an `<h1>`-`<h6>` element per the "Headings" section of
/// `translation_approach.md`.
pub(super) fn headings_handler(
    handlers: &dyn Handlers,
    element: &Element,
) -> Option<HandlerResult> {
    serialize_if_extra_attrs_or_inline!(handlers, element, 0);
    let level = element.tag.chars().nth(1).unwrap() as u32 - '0' as u32;
    // A heading is a leaf block: its children begin an inline context.
    let content = handlers.walk_children_content(element.node, Context::INLINE);
    let content = content.trim_document_whitespace();

    // Only pure mode gets here in an inline context, which holds no block and
    // so no heading spelling.
    if element.context.is_inline() {
        return Some(content.into());
    }

    // A setext heading leaves its content at the start of a line, where an HTML
    // block opening would dissolve the heading; an ATX heading's `#` is leaf
    // block syntax the block scan matches first, so it is safe. Empty content
    // has no setext spelling at all -- its underline is empty too, leaving
    // nothing behind -- so it falls back to ATX as well. See the "Headings" and
    // "Paragraphs" sections of `translation_approach.md`.
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
            push_escaping_closing_sequence(&mut result, content);
        }
        result.push_str("\n\n");
    }
    Some(result.into())
}

/// Writes ATX content, escaping a trailing `#` run CommonMark would read as its
/// [closing sequence](https://spec.commonmark.org/0.31.2/#atx-headings).
fn push_escaping_closing_sequence(output: &mut String, content: &str) {
    let before_run = content.trim_end_matches('#');
    let is_closing_sequence = before_run.len() < content.len()
        && (before_run.is_empty() || before_run.ends_with([' ', '\t']));
    output.push_str(before_run);
    if is_closing_sequence {
        output.push('\\');
    }
    output.push_str(&content[before_run.len()..]);
}
