use crate::{
    Element,
    element_handler::{HandlerResult, Handlers},
    options::HeadingStyle,
    serialize_if_faithful,
    text_util::TrimDocumentWhitespace,
};

pub(super) fn headings_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let level = element.tag.chars().nth(1).unwrap() as u32 - '0' as u32;
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_document_whitespace();
    let content = content.trim_matches('\n');

    let mut result = String::from("\n\n");
    if (level == 1 || level == 2)
        && handlers.options().heading_style == HeadingStyle::Setex
        && can_use_setext(content)
    {
        // Use the Setext heading style for h1 and h2
        result.push_str(content);
        result.push('\n');
        let ch = if level == 1 { "=" } else { "-" };
        result.push_str(&ch.repeat(content.chars().count()));
        result.push_str("\n\n");
    } else {
        result.push_str(&"#".repeat(level as usize));
        result.push(' ');
        result.push_str(content);
        result.push_str("\n\n");
    }
    Some(result.into())
}

/// Whether a heading holding `content` can be written in the Setext style.
///
/// The underline attaches to the *paragraph* above it, and a paragraph may span
/// several lines — so a first line holding only a hard break's backslash, or
/// only whitespace, is no obstacle: the underline still attaches to the whole
/// run. Two things are.
///
/// A blank line ends the paragraph, so the underline would attach to whatever
/// came after the blank line rather than to the heading, leaving the rest of the
/// content outside it.
///
/// A raw `<br>` opening the first line starts an [HTML block of type
/// 7](https://spec.commonmark.org/0.31.2/#html-blocks), which runs to the next
/// blank line and takes the underline with it. It has to *open* the line: a
/// `<br>` with anything ahead of it is an inline tag, and one on a later line is
/// inside a paragraph an HTML block cannot interrupt.
///
/// Nothing else disqualifies Setext, and ATX is not a free fallback — it is a
/// single-line syntax, so content holding a newline loses everything past the
/// first line to a paragraph of its own. Falling back where Setext would have
/// worked does not play safe; it breaks the heading a different way.
fn can_use_setext(content: &str) -> bool {
    if content.contains("\n\n") {
        return false;
    }
    let first_line = content.lines().next().unwrap_or_default().trim();
    !first_line
        .strip_prefix("<br")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['>', '/', ' ']))
}
