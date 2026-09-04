use crate::{
    Context, Element,
    dom_walker::{is_block_element, is_type_1_element},
    element_handler::{HandlerResult, Handlers},
    node_util::parent_tag_name_equals,
    text_util::frame_as_block,
};
use html5ever::serialize::{HtmlSerializer, SerializeOpts, Serializer, TraversalScope, serialize};
use markup5ever_rcdom::{NodeData, SerializableHandle};
use phf::phf_set;
use std::io::{self, Write};

/// Returns from the enclosing handler with `content` as the element's HTML,
/// but only in faithful mode and only when `condition` holds. Pure mode has no
/// HTML to fall back on, so it always translates.
macro_rules! serialize_when_faithful {
    ($handlers:expr, $condition:expr, $content:expr) => {
        if $handlers.options().translation_mode == $crate::options::TranslationMode::Faithful
            && $condition
        {
            return Some($crate::element_handler::HandlerResult::html($content));
        }
    };
}

pub(crate) use serialize_when_faithful;

/// Handles a structural element when it has an allowed parent, or preserves it
/// as HTML in faithful mode when it appears elsewhere.
///
/// Every element handled here is part of a CommonMark block, so faithful mode
/// also writes it as HTML when it appears in an inline context, where no block
/// may begin; see the "Translating HTML nodes" section of
/// `unsupported_html.md`. It does the same when the element carries more
/// attributes than its Markdown translation can express, on the same terms as
/// [`serialize_if_extra_attrs!`]: `num_attrs_allowed` of -1 rejects every
/// attribute set, and [`i64::MAX`] accepts any. Either test serializes the same
/// element, so the order they are tried in does not matter.
///
/// Handled child content is framed as a block, and translated in
/// `children_context`. When `propagate_children_translation` is true, the
/// returned translation status is false if any child required HTML; callers can
/// use that status to fall back to serializing a larger containing structure.
pub(super) fn handle_or_serialize_by_parent(
    handlers: &dyn Handlers,
    element: &Element,
    tag_names: &[&str],
    num_attrs_allowed: i64,
    propagate_children_translation: bool,
    children_context: Context,
) -> Option<HandlerResult> {
    serialize_when_faithful!(
        handlers,
        element.context == Context::Inline
            || element.attrs.len() as i64 > num_attrs_allowed
            || !parent_tag_name_equals(element.node, tag_names),
        serialize_element(handlers, element)
    );
    let result = handlers.walk_children(element.node, children_context);
    Some(HandlerResult {
        content: frame_as_block(&result.content),
        markdown_translated: !propagate_children_translation || result.markdown_translated,
    })
}

/// The [`HandlerResult`] for an element which can only be written as HTML: the
/// element serialized per its [`Context`], reported as not translated so that a
/// container needing all-CommonMark children can serialize itself instead.
pub(crate) fn serialize_element_result(
    handlers: &dyn Handlers,
    element: &Element,
) -> HandlerResult {
    HandlerResult::html(serialize_element(handlers, element))
}

pub(crate) fn serialize_element(handlers: &dyn Handlers, element: &Element) -> String {
    try_serialize_element(handlers, element).unwrap_or_else(|error| error.to_string())
}

fn try_serialize_element(handlers: &dyn Handlers, element: &Element) -> io::Result<String> {
    // An element which can't be translated to CommonMark is an HTML block only
    // where a block may begin *and* its tag opens one. A tag outside the type 1
    // and 6 lists would instead open a type 7 HTML block, which cannot
    // interrupt a paragraph and which swallows every following line down to the
    // next blank one; a raw HTML inline is what such a tag needs, whatever the
    // context. See the "Translating HTML nodes" section of
    // `unsupported_html.md`.
    if element.context == Context::Block && is_block_element(element.tag) {
        serialize_block_element(element)
    } else if is_raw_text_element(element.tag) {
        // What a raw text element holds is literal characters, not markup, so
        // translating it the way `serialize_inline_element` does would rewrite
        // the text itself.
        serialize_inline_verbatim(element)
    } else {
        serialize_inline_element(handlers, element)
    }
}

/// The tags whose content an HTML parser reads as raw text: no markup, and no
/// character references either. That second half is what rules out translating
/// them; see [`serialize_inline_verbatim`]. The RCDATA elements `textarea` and
/// `title` are deliberately absent — an HTML parser does decode references
/// inside those, so they take the ordinary raw HTML inline path, where the
/// Markdown their content holds is escaped along with it.
///
/// **In an inline context that path does not meet the newline encoding the
/// "Translating HTML nodes" table of `unsupported_html.md` asks of these two
/// tags** — type 1 for `textarea`, type 6 for `title`. Walking the content as
/// CommonMark is what escapes the Markdown in it, and that same walk compresses
/// a line ending to a space before anything can encode it:
/// `<h1><textarea>a⏎b</textarea></h1>` is written `# <textarea>a b</textarea>`,
/// not `# <textarea>a&#10;b</textarea>`. Keeping both halves would need a walk
/// which preserves whitespace *and* escapes Markdown specials, and neither mode
/// the walker has does both — `is_pre_element` content preserves whitespace but
/// goes out unescaped. The line ending is what is given up, because giving up
/// the escaping instead would let `a*b*c` in a textarea come back as emphasis.
static RAW_TEXT_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "script", "style",
};

fn is_raw_text_element(tag: &str) -> bool {
    RAW_TEXT_ELEMENTS.contains(tag)
}

fn serialize_opts() -> SerializeOpts {
    SerializeOpts {
        traversal_scope: TraversalScope::IncludeNode,
        ..Default::default()
    }
}

fn serialize_inline_element(handlers: &dyn Handlers, element: &Element) -> io::Result<String> {
    let NodeData::Element { name, attrs, .. } = &element.node.data else {
        return Err(io::Error::other("expected an element node"));
    };

    let mut bytes = Vec::new();
    let mut serializer = HtmlSerializer::new(&mut bytes, serialize_opts());
    serializer.start_elem(
        name.clone(),
        attrs
            .borrow()
            .iter()
            .map(|attr| (&attr.name, &attr.value[..])),
    )?;
    serializer
        .writer
        // What a raw HTML inline holds is inline content, whatever context the
        // element itself appears in.
        .write_all(
            handlers
                .walk_children_content(element.node, Context::Inline)
                .as_bytes(),
        )?;
    serializer.end_elem(name.clone())?;
    let html = String::from_utf8(bytes).map_err(io::Error::other)?;
    Ok(escape_inline_line_endings(html))
}

fn serialize_block_element(element: &Element) -> io::Result<String> {
    let html = serialize_subtree(element)?;
    // A type 6 HTML block ends at the next blank line, so a blank line written
    // inside the serialized HTML has to be escaped to keep the block in one
    // piece. A type 1 block ends at the line holding its closing tag instead,
    // which leaves its line endings — blank ones included — free to stand as
    // they are; escaping them would rewrite the script, style, or preformatted
    // text itself. See the "Translating HTML nodes" section of
    // `unsupported_html.md`.
    let html = if is_type_1_element(element.tag) {
        html
    } else {
        escape_html_block_blank_lines(html)
    };
    Ok(frame_as_block(&html))
}

/// [`serialize_inline_verbatim`] for callers outside this module.
pub(crate) fn serialize_element_verbatim(element: &Element) -> String {
    serialize_inline_verbatim(element).unwrap_or_else(|error| error.to_string())
}

/// Serializes `element` and everything it holds as HTML, translating none of
/// it. Unlike a raw HTML inline, whose content is still translated, the content
/// of a code block or a raw text element is literal text: no CommonMark
/// encoding survives there. The result is a raw HTML inline, so its line
/// endings are escaped as well.
///
/// A line ending does survive this: only the tags of a raw HTML inline are
/// HTML, so a CommonMark parser reads what sits between them as text and
/// decodes the `&#13;`/`&#10;` there before any HTML parser sees a `<script>`
/// element.
///
/// **Limitation:** that same fact costs the content its escaping. Serializing
/// it leaves any Markdown special it holds bare, and the CommonMark parser
/// reading it back takes that special as Markdown — `a*b*c` in a script comes
/// back as `a<em>b</em>c`. Escaping the content instead would need an escape
/// which survives an HTML parser, and a character reference is not one: inside
/// a raw text element it stays those five or six characters. Representing this
/// faithfully needs the containing block serialized instead, which is the
/// "Special case" section of `unsupported_html.md`.
fn serialize_inline_verbatim(element: &Element) -> io::Result<String> {
    serialize_subtree(element).map(escape_inline_line_endings)
}

fn serialize_subtree(element: &Element) -> io::Result<String> {
    let mut bytes = Vec::new();
    let handle = SerializableHandle::from(element.node.clone());
    serialize(&mut bytes, &handle, serialize_opts())?;
    String::from_utf8(bytes).map_err(io::Error::other)
}

// A raw HTML inline lives inside a leaf block, and every leaf block is ended by
// a line ending it does not expect: a blank line ends the paragraph holding one,
// a single line ending ends an ATX heading or a table row. Encode every line
// ending, the safe over-generalization of the blank line rule described in
// `unsupported_html.md`. A character reference in a raw HTML inline's tag or in
// the CommonMark text it holds decodes back to the line ending it replaced.
pub(crate) fn escape_inline_line_endings(html: String) -> String {
    if !has_line_ending(&html) {
        return html;
    }

    let mut result = String::with_capacity(html.len());
    for ch in html.chars() {
        match ch {
            '\r' => result.push_str("&#13;"),
            '\n' => result.push_str("&#10;"),
            _ => result.push(ch),
        }
    }
    result
}

/// Whether `html` holds anything for the escapes below to do: the fast path
/// both of them share, and the one place the set of line endings they know
/// about is written down.
fn has_line_ending(html: &str) -> bool {
    html.contains(['\r', '\n'])
}

// A blank line terminates a CommonMark HTML block. Encode every line ending
// after the first so serialized block content remains in one HTML block.
fn escape_html_block_blank_lines(html: String) -> String {
    if !has_line_ending(&html) {
        return html;
    }

    let mut result = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\r' && ch != '\n' {
            result.push(ch);
            continue;
        }

        result.push(ch);
        if ch == '\r' && chars.peek() == Some(&'\n') {
            result.push(chars.next().unwrap());
        }

        copy_horizontal_whitespace(&mut chars, &mut result);
        let Some(next) = chars.next() else {
            break;
        };
        match next {
            '\n' => result.push_str("&#10;"),
            '\r' if chars.peek() == Some(&'\n') => {
                chars.next();
                result.push_str("&#13;&#10;");
            }
            '\r' => result.push_str("&#13;"),
            _ => {
                result.push(next);
                continue;
            }
        }
        copy_horizontal_whitespace(&mut chars, &mut result);
    }

    result
}

fn copy_horizontal_whitespace<I>(chars: &mut std::iter::Peekable<I>, output: &mut String)
where
    I: Iterator<Item = char>,
{
    while let Some(&next) = chars.peek() {
        if !next.is_whitespace() || next == '\r' || next == '\n' {
            break;
        }
        output.push(next);
        chars.next();
    }
}

/// Returns from the enclosing handler with `element` written as HTML when it
/// carries more attributes than its Markdown translation can express.
/// `num_attrs_allowed` of -1 rejects every attribute set, serializing the
/// element whenever the mode is faithful; [`i64::MAX`] accepts any.
macro_rules! serialize_if_extra_attrs {
    ($handlers:expr, $element:expr, $num_attrs_allowed:expr) => {
        $crate::element_handler::element_util::serialize_when_faithful!(
            $handlers,
            $element.attrs.len() as i64 > $num_attrs_allowed,
            $crate::element_handler::element_util::serialize_element($handlers, &$element)
        )
    };
}

pub(crate) use serialize_if_extra_attrs;

/// [`serialize_if_extra_attrs!`] for a handler whose Markdown is a CommonMark
/// block, which additionally needs a block context: a block can only be written
/// where one may begin, so in an inline context the element is written as a raw
/// HTML inline instead. See the "Translating HTML nodes" section of
/// `unsupported_html.md`. Either test serializes the same element, so the order
/// they are tried in does not matter.
macro_rules! serialize_if_extra_attrs_or_inline {
    ($handlers:expr, $element:expr, $num_attrs_allowed:expr) => {
        $crate::element_handler::element_util::serialize_when_faithful!(
            $handlers,
            $element.context == $crate::Context::Inline
                || $element.attrs.len() as i64 > $num_attrs_allowed,
            $crate::element_handler::element_util::serialize_element($handlers, &$element)
        )
    };
}

pub(crate) use serialize_if_extra_attrs_or_inline;

#[cfg(test)]
mod tests {
    use super::{escape_html_block_blank_lines, escape_inline_line_endings};

    #[test]
    fn escapes_every_line_ending_of_a_raw_inline() {
        assert_eq!("ab", escape_inline_line_endings("ab".into()));
        assert_eq!("a&#10;b", escape_inline_line_endings("a\nb".into()));
        assert_eq!("a&#13;&#10;b", escape_inline_line_endings("a\r\nb".into()));
        assert_eq!("a&#13;b", escape_inline_line_endings("a\rb".into()));
    }

    #[test]
    fn escapes_blank_lines_for_each_line_ending_style() {
        assert_eq!("a\n&#10;b", escape_html_block_blank_lines("a\n\nb".into()));
        assert_eq!(
            "a\r\n&#13;&#10;b",
            escape_html_block_blank_lines("a\r\n\r\nb".into())
        );
        assert_eq!("a\r&#13;b", escape_html_block_blank_lines("a\r\rb".into()));
    }
}
