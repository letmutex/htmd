use std::rc::Rc;

use markup5ever_rcdom::{Node, NodeData};

use crate::{
    Element,
    dom_walker::is_block_element,
    element_handler::{HandlerResult, Handlers, element_util::serialize_element},
    node_util::{get_node_tag_name, get_parent_node},
    options::TranslationMode,
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn emphasis_handler(
    handlers: &dyn Handlers,
    element: Element,
    marker: &str,
) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let content = handlers.walk_children(element.node).content;
    if content.is_empty() {
        return None;
    }
    let is_pure = handlers.options().translation_mode == TranslationMode::Pure;

    // Note: this is whitespace, NOT document whitespace, per the
    // [Commonmark spec](https://spec.commonmark.org/0.31.2/#emphasis-and-strong-emphasis).
    let leading_len = leading_hoist(&content, is_pure);
    let (leading, rest) = content.split_at(leading_len);
    let trailing_len = trailing_hoist(rest, is_pure);
    let (rest, trailing) = rest.split_at(rest.len() - trailing_len);

    if rest.is_empty() {
        // Nothing is left for the markers to wrap, so none are written. A hard
        // break that moved out still has to be: it was the element's whole
        // content, and dropping it would lose the `<br>` it came from. Plain
        // whitespace goes, as it always has.
        return if leading.contains('\n') || trailing.contains('\n') {
            Some(concat_strings!(leading, trailing).into())
        } else {
            None
        };
    }

    // Markers that do not flank are literal text rather than emphasis, so the
    // element survives only as HTML. Pure mode has no such fallback; the hoist
    // above is what keeps a marker off a break there.
    if !is_pure && !markers_flank(element.node, rest, leading, trailing) {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }

    Some(concat_strings!(leading, marker, rest, marker, trailing).into())
}

/// How much of `content`'s start has to be written outside the opening marker.
///
/// Leading whitespace always moves out, since a marker against whitespace does
/// not open emphasis. A leading hard break moves too, but only in pure mode: the
/// break renders the same on either side of the markers, while in faithful mode
/// moving it would move the `<br>` it came from out of the element — there the
/// break instead makes the markers unwritable and the element is serialized.
///
/// A blank line is block separation rather than a break, so it moves out in both
/// modes.
///
/// A newline here means `\n` and nothing else: html5ever normalizes the source's
/// CR and CRLF away, so a `\r` reaching this is a literal character carried
/// through a `<pre>` verbatim, not a line ending, and treating it as one would
/// cut the hoist short.
fn leading_hoist(content: &str, is_pure: bool) -> usize {
    let whitespace_len = content.len() - content.trim_start().len();
    if !is_pure {
        let whitespace = &content[..whitespace_len];
        return match whitespace.find('\n') {
            Some(_) if whitespace.contains("\n\n") => whitespace_len,
            Some(break_start) => break_start,
            None => whitespace_len,
        };
    }
    // Take any run of hard breaks along with the whitespace. Only the backslash
    // spelling needs looking for: the two-space one is whitespace already.
    let mut len = whitespace_len;
    while content[len..].starts_with('\\') && content[len + 1..].starts_with('\n') {
        let after = &content[len + 1..];
        len += 1 + (after.len() - after.trim_start().len());
    }
    len
}

/// How much of `content`'s end has to be written outside the closing marker,
/// the mirror of [`leading_hoist`].
fn trailing_hoist(content: &str, is_pure: bool) -> usize {
    let trimmed_len = content.trim_end().len();
    if !is_pure {
        let whitespace = &content[trimmed_len..];
        return match whitespace.rfind('\n') {
            Some(_) if whitespace.contains("\n\n") => whitespace.len(),
            // Only what follows the break's newline may move out, which for a
            // break the content ends on is nothing at all.
            Some(break_end) => whitespace.len() - break_end - 1,
            None => whitespace.len(),
        };
    }
    let mut end = trimmed_len;
    while content[end..].starts_with('\n') && ends_in_break_marker(&content[..end]) {
        end = content[..end - 1].trim_end().len();
    }
    content.len() - end
}

/// Whether the backslash `text` ends on is a hard break's own marker rather than
/// half of an escaped literal backslash.
///
/// Both look alike from the newline that follows, so only the run's length tells
/// them apart: escaping doubles every literal, and a break's own marker is the
/// one extra that leaves the run odd. Getting this wrong is not cosmetic —
/// hoisting one backslash out of an escaped pair leaves the other escaping the
/// closing marker, losing the emphasis entirely.
pub(super) fn ends_in_break_marker(text: &str) -> bool {
    // Backslash is ASCII, so counting bytes cannot split a character.
    text.bytes().rev().take_while(|&byte| byte == b'\\').count() % 2 == 1
}

/// Whether the markers this element would be written with open and close
/// emphasis where they land.
///
/// A delimiter run opens emphasis only if it is
/// [left-flanking](https://spec.commonmark.org/0.31.2/#left-flanking-delimiter-run):
/// not followed by whitespace, and either not followed by punctuation or else
/// itself preceded by whitespace or punctuation — mirror-image for closing.
/// `rest` is what the markers would wrap, so its edges are what they sit against
/// on the inside; a non-empty `leading` or `trailing` is whitespace that moved
/// out, so the markers sit against whitespace on the outside.
///
/// Every spelling of a `<br>` puts punctuation at the content's edge — `<` and
/// `>` for the raw tag, `\` for the backslash break, whitespace for the
/// two-space one — which is why a break is what usually brings this test down.
fn markers_flank(node: &Rc<Node>, rest: &str, leading: &str, trailing: &str) -> bool {
    let first = rest.chars().next().expect("rest is not empty");
    let last = rest.chars().last().expect("rest is not empty");
    let opens = !first.is_whitespace()
        && (first.is_alphanumeric()
            || !leading.is_empty()
            || flanking_allows(preceding_char(node)));
    let closes = !last.is_whitespace()
        && (last.is_alphanumeric()
            || !trailing.is_empty()
            || flanking_allows(following_char(node)));
    opens && closes
}

/// Whether a marker against punctuation may still flank, given the character on
/// its other side. Anything but an alphanumeric will do — whitespace and
/// punctuation both allow it, and so does the line or block edge a `None`
/// stands for.
fn flanking_allows(adjacent: Option<char>) -> bool {
    !adjacent.is_some_and(char::is_alphanumeric)
}

/// What a walk out from an element toward one end of its line ran into.
enum Adjacent {
    /// The character the output holds there.
    Char(char),
    /// A line or block edge, which flanking counts as whitespace.
    Edge,
    /// Nothing that reaches the output; keep looking further out.
    Nothing,
}

/// The character the output holds ahead of this element, or `None` where the
/// element opens a line or a block.
fn preceding_char(node: &Rc<Node>) -> Option<char> {
    adjacent_char(node, Direction::Back)
}

/// The character the output holds after this element, or `None` where the
/// element ends a line or a block.
fn following_char(node: &Rc<Node>) -> Option<char> {
    adjacent_char(node, Direction::Forward)
}

/// Which way out of an element to look for the character next to it.
#[derive(Clone, Copy)]
enum Direction {
    Back,
    Forward,
}

/// Walks out from `node` through the inline elements around it, scanning each
/// element's siblings on the given side, until one of them reaches the output or
/// a block boundary ends the search.
fn adjacent_char(node: &Rc<Node>, direction: Direction) -> Option<char> {
    let mut current = node.clone();
    while let Some(parent) = get_parent_node(&current) {
        let found = {
            let children = parent.children.borrow();
            let index = children.iter().position(|c| Rc::ptr_eq(c, &current))?;
            match direction {
                Direction::Back => scan_back(&children[..index]),
                Direction::Forward => scan_forward(&children[index + 1..]),
            }
        };
        match found {
            Adjacent::Char(ch) => return Some(ch),
            Adjacent::Edge => return None,
            Adjacent::Nothing => {}
        }
        // Nothing lies that way within this element. The line reaches past it
        // only while it is inline: a block element starts a line of its own.
        if get_node_tag_name(&parent).is_none_or(is_block_element) {
            return None;
        }
        current = parent;
    }
    None
}

/// Scans `nodes` — the siblings ahead of an element, in document order — back to
/// front, stopping at the first one that reaches the output.
fn scan_back(nodes: &[Rc<Node>]) -> Adjacent {
    for node in nodes.iter().rev() {
        match &node.data {
            NodeData::Text { contents } => {
                let last = contents.borrow().chars().last();
                if let Some(ch) = last {
                    return Adjacent::Char(ch);
                }
            }
            NodeData::Element { name, .. } => {
                let tag = &*name.local;
                if tag == "br" || is_block_element(tag) {
                    // Both end the line they sit on.
                    return Adjacent::Edge;
                }
                if tag == "img" {
                    // Childless, but it still writes a link, which ends in `)`.
                    return Adjacent::Char(')');
                }
                // An element writing markers of its own puts punctuation here
                // rather than the text this finds, which only makes the answer
                // stricter: the element is serialized where it need not have
                // been, never written where it cannot be.
                match scan_back(&node.children.borrow()) {
                    Adjacent::Nothing => {}
                    found => return found,
                }
            }
            _ => {}
        }
    }
    Adjacent::Nothing
}

/// Scans `nodes` — the siblings after an element, in document order — front to
/// back, the mirror of [`scan_back`].
fn scan_forward(nodes: &[Rc<Node>]) -> Adjacent {
    for node in nodes {
        match &node.data {
            NodeData::Text { contents } => {
                let first = contents.borrow().chars().next();
                if let Some(ch) = first {
                    return Adjacent::Char(ch);
                }
            }
            NodeData::Element { name, .. } => {
                let tag = &*name.local;
                if tag == "br" || is_block_element(tag) {
                    return Adjacent::Edge;
                }
                if tag == "img" {
                    // The link it writes starts with `!`.
                    return Adjacent::Char('!');
                }
                match scan_forward(&node.children.borrow()) {
                    Adjacent::Nothing => {}
                    found => return found,
                }
            }
            _ => {}
        }
    }
    Adjacent::Nothing
}

#[cfg(test)]
mod tests {
    use super::{leading_hoist, trailing_hoist};

    /// A `\r` is whitespace, so it moves out with the rest of the run, but it is
    /// not a *line ending*, so it never marks the break position the hoists stop
    /// at. The end-to-end behavior is identical either way, so only the hoists'
    /// own contract can pin the distinction.
    #[test]
    fn carriage_return_is_not_a_line_ending() {
        // Faithful mode: the whole whitespace run moves out, as it would for a
        // run holding no line ending at all.
        assert_eq!(1, leading_hoist("\ra", false));
        assert_eq!(1, trailing_hoist("a\r", false));

        // A real `\n` in the run still stops it, `\r` or no `\r`.
        assert_eq!(1, leading_hoist("\r\na", false));
        assert_eq!(0, trailing_hoist("a\r\n", false));

        // Pure mode: a `\` before a `\r` is a literal backslash next to a
        // carriage return, not a backslash hard break, so nothing is taken.
        assert_eq!(0, leading_hoist("\\\ra", true));
        assert_eq!(1, trailing_hoist("a\\\r", true));

        // The same `\` before a real `\n` is a break, and is taken.
        assert_eq!(2, leading_hoist("\\\na", true));
        assert_eq!(2, trailing_hoist("a\\\n", true));
    }

    /// `trailing_backslash_is_not_a_hard_break` in `basic_tests.rs` pins the
    /// consequence end to end; only these pin the odd/even boundary itself.
    #[test]
    fn an_escaped_backslash_is_not_a_break_marker() {
        // Even runs: the newline alone moves out, leaving every backslash of
        // the pair inside the markers.
        assert_eq!(1, trailing_hoist(concat!(r"a\\", "\n"), true));
        assert_eq!(1, trailing_hoist(concat!(r"a\\\\", "\n"), true));

        // Odd runs: the unpaired backslash is the break's marker and moves out
        // with the newline — just the one, however long the run.
        assert_eq!(2, trailing_hoist(concat!(r"a\", "\n"), true));
        assert_eq!(2, trailing_hoist(concat!(r"a\\\", "\n"), true));
    }
}
