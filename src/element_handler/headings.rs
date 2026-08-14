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
/// heading's text as well. This writes each break the way `br_handler` would
/// have written it had it known the heading could not use Setext: as the `<br>`
/// `raw_br_or_drop` falls back to in faithful mode, and as nothing at all in
/// pure mode, which is what dropping the `<br>` leaves.
/// The result is the output [`HeadingStyle::Atx`] gives for the same input, so
/// choosing Setext is never worse than not choosing it.
///
/// # A `<pre>` is caught along with the breaks
///
/// Only the two spellings tell a break apart from ordinary text, and a `<pre>`
/// keeps its own whitespace, so a line of one ending in two spaces reads as a
/// break here and is folded like one. That costs nothing in practice: a `<pre>`
/// is a block, so a heading holding one has a blank line in its content, which
/// is exactly what sent it down this branch — and a block inside a heading is
/// already past what either heading syntax can express. Narrowing this to the
/// content ahead of the first blank line would spare the `<pre>` but leave a
/// real break behind whenever one follows a block, which is the worse half of
/// the trade.
///
/// The `<br>` written back is bare because only a bare one reaches here: the
/// `serialize_if_faithful!` at the top of `br_handler` has already turned any
/// `<br>` carrying an attribute into raw HTML, which no break spelling touches.
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
/// The underline attaches to the *paragraph* above it, and a paragraph may span
/// several lines — so a first line holding only a hard break's backslash, or
/// only whitespace, is no obstacle: the underline still attaches to the whole
/// run. Two things are.
///
/// A blank line ends the paragraph, so the underline would attach to whatever
/// came after the blank line rather than to the heading, leaving the rest of the
/// content outside it. A line of nothing but spaces or tabs is
/// [blank](https://spec.commonmark.org/0.31.2/#blank-lines) as much as an empty
/// one is, and one does reach here: whitespace is compressed away everywhere
/// else, but a `<pre>` keeps its own verbatim. `content` arrives trimmed at both
/// ends, so every blank line found this way is an interior one.
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

        // A line with something on it is not blank, whatever surrounds it, and
        // a no-break space is not whitespace a blank line is made of.
        assert!(can_use_setext("a\nb"));
        assert!(can_use_setext("a\n \u{a0} \nb"));
        assert!(can_use_setext(""));

        // A `\r\n` is a line ending like any other, so the empty line it leaves
        // behind is blank all the same. html5ever normalizes the source's own
        // carriage returns away, but a `<pre>` carries one through verbatim.
        assert!(!can_use_setext("a\n\r\nb"));
    }
}
