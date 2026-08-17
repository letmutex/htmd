use std::borrow::Cow;

use crate::{
    Element,
    element_handler::{HandlerResult, Handlers, emphasis::ends_in_break_marker},
    options::{HeadingStyle, TranslationMode},
    serialize_if_faithful,
    text_util::TrimDocumentWhitespace,
};

pub(super) fn headings_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let level = element.tag.chars().nth(1).unwrap() as u32 - '0' as u32;
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_document_whitespace();
    let content = content.trim_matches('\n');

    // Whether `br_handler` wrote this heading's breaks as hard line breaks. It
    // asks only the level and the style, since it cannot see the content that
    // `can_use_setext` judges — the walk above is what produces it.
    let writes_hard_breaks =
        (level == 1 || level == 2) && handlers.options().heading_style == HeadingStyle::Setex;

    let mut result = String::from("\n\n");
    if writes_hard_breaks && can_use_setext(content) {
        // Use the Setext heading style for h1 and h2
        result.push_str(content);
        result.push('\n');
        let ch = if level == 1 { "=" } else { "-" };
        result.push_str(&ch.repeat(content.chars().count()));
        result.push_str("\n\n");
    } else {
        let content = if writes_hard_breaks {
            Cow::Owned(fold_hard_breaks(content, handlers))
        } else {
            Cow::Borrowed(content)
        };
        result.push_str(&"#".repeat(level as usize));
        result.push(' ');
        result.push_str(&content);
        result.push_str("\n\n");
    }
    Some(result.into())
}

/// Rewrites the hard line breaks in `content` as an ATX heading needs them.
///
/// An ATX heading is a single line, so a hard break ends the heading and leaves
/// the rest of the content to a paragraph of its own — and under
/// [`BrStyle::Backslash`](crate::options::BrStyle) it leaves a stray `\` in the
/// heading's text as well. Each break is rewritten the way `br_handler` would
/// have written it had it known Setext was unavailable, so the result matches
/// what [`HeadingStyle::Atx`] gives for the same input.
///
/// A `<pre>` line ending in two spaces is folded as if it were a break, since
/// only the spellings tell the two apart. That costs nothing: a `<pre>` is a
/// block, so its blank line is what sent the heading down this branch in the
/// first place, and a block inside a heading is already past what either heading
/// syntax can express.
///
/// The `<br>` written back is bare because only a bare one reaches here —
/// `serialize_if_faithful!` in `br_handler` turns any `<br>` with an attribute
/// into raw HTML, which no break spelling touches.
fn fold_hard_breaks(content: &str, handlers: &dyn Handlers) -> String {
    let replacement = if handlers.options().translation_mode == TranslationMode::Faithful {
        "<br>"
    } else {
        ""
    };
    let mut result = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(newline) = rest.find('\n') {
        let line = &rest[..newline];
        // The two-space spelling is whatever the line ends in; the backslash one
        // is a break only where the run it ends is odd, since an even run is
        // escaped literal backslashes and the newline is the block's own.
        let text = line
            .strip_suffix("  ")
            .or_else(|| ends_in_break_marker(line).then(|| &line[..line.len() - 1]));
        match text {
            Some(text) => {
                result.push_str(text);
                result.push_str(replacement);
            }
            None => {
                result.push_str(line);
                result.push('\n');
            }
        }
        rest = &rest[newline + 1..];
    }
    result.push_str(rest);
    result
}

/// Whether a heading holding `content` can be written in the Setext style.
///
/// The underline attaches to the whole *paragraph* above it, so multiple lines
/// are fine. Only two things disqualify Setext:
///
/// - A [blank line](https://spec.commonmark.org/0.31.2/#blank-lines) ends that
///   paragraph, leaving the underline attached to what follows instead. Spaces
///   and tabs count as blank, and such a line does reach here: whitespace is
///   compressed away everywhere else, but a `<pre>` keeps its own verbatim.
///   `content` arrives trimmed, so any blank line found is an interior one.
/// - A raw `<br>` *opening* the first line starts an [HTML block of type
///   7](https://spec.commonmark.org/0.31.2/#html-blocks), which runs to the next
///   blank line and takes the underline with it. A `<br>` with anything ahead of
///   it is an inline tag, and one on a later line is inside a paragraph an HTML
///   block cannot interrupt.
///
/// ATX is not a free fallback — being single-line, it loses everything past the
/// first line to a paragraph — so falling back where Setext would have worked
/// breaks the heading a different way rather than playing safe.
fn can_use_setext(content: &str) -> bool {
    if content
        .lines()
        .any(|line| line.trim_matches([' ', '\t']).is_empty())
    {
        return false;
    }
    let first_line = content.lines().next().unwrap_or_default().trim();
    !first_line
        .strip_prefix("<br")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['>', '/', ' ']))
}

#[cfg(test)]
mod tests {
    use super::can_use_setext;

    /// A blank line is one holding nothing but spaces and tabs, so looking for
    /// an empty one alone misses the rest. `setext_falls_back_to_atx_only_for_a_blank_line`
    /// in `basic_tests.rs` carries the `<pre>` that reaches this end to end.
    #[test]
    fn a_whitespace_only_line_is_blank() {
        assert!(!can_use_setext("a\n   \nb"));
        assert!(!can_use_setext("a\n\t\nb"));
        assert!(!can_use_setext("a\n \t \nb"));
        assert!(!can_use_setext("a\n\nb"));

        // A no-break space is not whitespace a blank line is made of.
        assert!(can_use_setext("a\nb"));
        assert!(can_use_setext("a\n \u{a0} \nb"));
        assert!(can_use_setext(""));

        // html5ever normalizes the source's own carriage returns away, but a
        // `<pre>` carries one through verbatim.
        assert!(!can_use_setext("a\n\r\nb"));
    }
}
