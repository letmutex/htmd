use crate::{
    Context, Element,
    dom_walker::{is_block_element, is_type_1_element},
    element_handler::{HandlerResult, Handlers},
    node_util::parent_tag_name_equals,
    text_util::{frame_as_block, has_line_ending, push_encoding_line_ending},
};
use html5ever::serialize::{HtmlSerializer, SerializeOpts, Serializer, TraversalScope, serialize};
use markup5ever_rcdom::{NodeData, SerializableHandle};
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
        element.context.is_inline()
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
    } else {
        serialize_inline_element(handlers, element)
    }
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
    // Everything written so far is the open tag, whose attribute values hold
    // the only whitespace of this element the walk below does not reach.
    let open_tag_len = serializer.writer.len();
    serializer.writer.write_all(
        handlers
            .walk_raw_html_inline_children(element.node)
            .as_bytes(),
    )?;
    serializer.end_elem(name.clone())?;
    let mut html = String::from_utf8(bytes).map_err(io::Error::other)?;
    if has_line_ending(&html[..open_tag_len]) {
        let encoded = escape_attribute_line_endings(&html[..open_tag_len]);
        html.replace_range(..open_tag_len, &encoded);
    }
    Ok(html)
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

fn serialize_subtree(element: &Element) -> io::Result<String> {
    let mut bytes = Vec::new();
    let handle = SerializableHandle::from(element.node.clone());
    serialize(&mut bytes, &handle, serialize_opts())?;
    String::from_utf8(bytes).map_err(io::Error::other)
}

/// Writes every line ending in the open tag of a raw HTML inline as a
/// character reference.
///
/// A raw HTML inline lives inside a leaf block, which a line ending ends: a
/// blank one ends a paragraph, a single one an ATX heading or a table row. The
/// contents are kept safe by the whitespace collapsing of the walk, but
/// `unsupported_html.md` leaves an attribute value's whitespace alone, so a
/// line ending there is encoded instead — the open tag passes through
/// CommonMark verbatim, and the HTML parser reading the result decodes it back.
fn escape_attribute_line_endings(open_tag: &str) -> String {
    let mut result = String::with_capacity(open_tag.len());
    for ch in open_tag.chars() {
        push_encoding_line_ending(&mut result, ch);
    }
    result
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

        copy_blank_line_whitespace(&mut chars, &mut result);
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
        copy_blank_line_whitespace(&mut chars, &mut result);
    }

    result
}

/// Copies the run of spaces and tabs at `chars` — the only characters a
/// [blank line](https://spec.commonmark.org/0.31.2/#characters-and-lines) may
/// hold, so a line carrying any other whitespace ends no HTML block.
fn copy_blank_line_whitespace<I>(chars: &mut std::iter::Peekable<I>, output: &mut String)
where
    I: Iterator<Item = char>,
{
    while let Some(&next) = chars.peek() {
        if next != ' ' && next != '\t' {
            break;
        }
        output.push(next);
        chars.next();
    }
}

/// Returns from the enclosing handler with `element` written as HTML when
/// `condition` holds and the mode is faithful. The macros below are spellings
/// of this with the condition filled in.
macro_rules! serialize_element_when_faithful {
    ($handlers:expr, $element:expr, $condition:expr) => {
        $crate::element_handler::element_util::serialize_when_faithful!(
            $handlers,
            $condition,
            $crate::element_handler::element_util::serialize_element($handlers, &$element)
        )
    };
}

pub(crate) use serialize_element_when_faithful;

/// [`serialize_element_when_faithful!`] for a handler which has already walked
/// its children. Writing the element as HTML throws that walk away, so the link
/// reference definitions it buffered are rolled back to `checkpoint` first. See
/// [`anchor::LinkReferenceCheckpoint`].
///
/// [`anchor::LinkReferenceCheckpoint`]:
///     crate::element_handler::anchor::LinkReferenceCheckpoint
macro_rules! serialize_walked_element_when_faithful {
    ($handlers:expr, $element:expr, $condition:expr, $checkpoint:expr) => {
        if $handlers.options().translation_mode == $crate::options::TranslationMode::Faithful
            && $condition
        {
            $checkpoint.roll_back();
            return Some(
                $crate::element_handler::element_util::serialize_element_result(
                    $handlers, &$element,
                ),
            );
        }
    };
}

pub(crate) use serialize_walked_element_when_faithful;

/// Returns from the enclosing handler with `element` written as HTML when it
/// carries more attributes than its Markdown translation can express.
/// `num_attrs_allowed` of -1 rejects every attribute set, serializing the
/// element whenever the mode is faithful; [`i64::MAX`] accepts any.
macro_rules! serialize_if_extra_attrs {
    ($handlers:expr, $element:expr, $num_attrs_allowed:expr) => {
        $crate::element_handler::element_util::serialize_element_when_faithful!(
            $handlers,
            $element,
            $element.attrs.len() as i64 > $num_attrs_allowed
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
        $crate::element_handler::element_util::serialize_element_when_faithful!(
            $handlers,
            $element,
            $element.context.is_inline() || $element.attrs.len() as i64 > $num_attrs_allowed
        )
    };
}

pub(crate) use serialize_if_extra_attrs_or_inline;

#[cfg(test)]
mod tests {
    use super::{escape_attribute_line_endings, escape_html_block_blank_lines};

    #[test]
    fn escapes_the_line_endings_of_an_open_tag() {
        assert_eq!("ab", escape_attribute_line_endings("ab"));
        assert_eq!("a&#10;b", escape_attribute_line_endings("a\nb"));
        assert_eq!("a&#13;&#10;b", escape_attribute_line_endings("a\r\nb"));
        assert_eq!("a&#13;b", escape_attribute_line_endings("a\rb"));
        assert_eq!("a&#10;&#10;b", escape_attribute_line_endings("a\n\nb"));
        assert_eq!("a \t b", escape_attribute_line_endings("a \t b"));
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

    #[test]
    fn escapes_only_the_lines_which_are_blank() {
        assert_eq!(
            "a\n  &#10;b",
            escape_html_block_blank_lines("a\n  \nb".into())
        );
        assert_eq!(
            "a\n\t&#10;b",
            escape_html_block_blank_lines("a\n\t\nb".into())
        );
        assert_eq!(
            "a\n\u{0C}\nb",
            escape_html_block_blank_lines("a\n\u{0C}\nb".into())
        );
    }
}
