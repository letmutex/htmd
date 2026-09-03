use html5ever::tendril::{Tendril, fmt::UTF8};
use markup5ever_rcdom::{Node, NodeData};
use phf::phf_set;
use std::{borrow::Cow, cell::RefCell, rc::Rc};

use crate::element_handler::ElementHandlers;

use super::{
    options::TranslationMode,
    text_util::{
        TrimDocumentWhitespace, compress_whitespace, index_of_markdown_ordered_item_dot,
        is_markdown_atx_heading,
    },
};

/// The state a walk carries down through a subtree unchanged. `parent_tag` and
/// `trim_leading_spaces` are deliberately not part of it: [`walk_children`]
/// recomputes both for every child, while this describes the surroundings every
/// descendant shares.
#[derive(Clone, Copy)]
pub(crate) struct WalkState {
    /// Whether the node sits inside a `<pre>` or `<code>`, where whitespace is
    /// preserved and text goes out unescaped.
    pub(crate) is_pre: bool,
}

pub(crate) fn walk_node(
    node: &Rc<Node>,
    output: &mut String,
    handlers: &ElementHandlers,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    state: WalkState,
) -> bool {
    match &node.data {
        NodeData::Document => {
            let root = WalkState { is_pre: false };
            let _ = walk_children(node, output, handlers, true, root);
            trim_output_end(output);
            true
        }
        NodeData::Text { contents } => walk_text(
            contents.borrow().as_ref(),
            output,
            parent_tag,
            trim_leading_spaces,
            state.is_pre,
        ),
        NodeData::Element { name, attrs, .. } => walk_element(
            node,
            name.local.as_ref(),
            &attrs.borrow(),
            output,
            handlers,
            state,
        ),
        NodeData::Comment { contents } => {
            if handlers.options.translation_mode == TranslationMode::Faithful {
                output.push_str("<!--");
                output.push_str(contents);
                output.push_str("-->");
            }
            true
        }
        NodeData::Doctype { .. } => true,
        NodeData::ProcessingInstruction { .. } => true,
    }
}

fn walk_text(
    text: &str,
    output: &mut String,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    is_pre: bool,
) -> bool {
    if is_pre {
        let text = if parent_tag == Some("pre") {
            escape_pre_text_if_needed(Cow::Borrowed(text))
        } else {
            Cow::Borrowed(text)
        };
        output.push_str(text.as_ref());
        return true;
    }

    let output_ends_with_space = output.ends_with(' ');
    if is_plain_text(text) {
        let text = if trim_leading_spaces || (text.starts_with(' ') && output_ends_with_space) {
            text.trim_start_matches(' ')
        } else {
            text
        };
        output.push_str(text);
        return true;
    }

    let text = escape_if_needed(Cow::Borrowed(text));
    let text = compress_whitespace(text.as_ref());
    let text = if trim_leading_spaces || (text.starts_with(' ') && output_ends_with_space) {
        text.trim_start_matches(' ')
    } else {
        text.as_ref()
    };
    output.push_str(text);
    true
}

fn walk_element(
    node: &Rc<Node>,
    tag: &str,
    attrs: &[html5ever::Attribute],
    output: &mut String,
    handlers: &ElementHandlers,
    state: WalkState,
) -> bool {
    if is_passthrough_span(tag, attrs, handlers) {
        let mut content = String::new();
        let markdown_translated = walk_children(node, &mut content, handlers, false, state);
        trim_newlines(&mut content);
        append_normalized_content(output, content, state.is_pre);
        return markdown_translated;
    }

    if handlers.options.translation_mode == TranslationMode::Pure
        && !handlers.tag_to_handler_indices.contains_key(tag)
    {
        let mut content = String::new();
        let is_block = is_block_element(tag);
        let markdown_translated = walk_children(node, &mut content, handlers, is_block, state);
        append_normalized_content(output, content, state.is_pre);
        return markdown_translated;
    }

    let Some(result) = handlers.handle(node, tag, attrs, true, 0) else {
        return true;
    };
    if !result.content.is_empty() || tag != "head" {
        append_normalized_content(output, result.content, state.is_pre);
    }
    result.markdown_translated
}

fn is_passthrough_span(
    tag: &str,
    attrs: &[html5ever::Attribute],
    handlers: &ElementHandlers,
) -> bool {
    tag == "span"
        && handlers.options.translation_mode == TranslationMode::Pure
        && handlers
            .tag_to_handler_indices
            .get("span")
            .is_some_and(|indices| indices.len() == 1)
        && !is_math_span(attrs)
}

fn trim_newlines(content: &mut String) {
    let start = content.len() - content.trim_start_matches('\n').len();
    if start > 0 {
        content.drain(..start);
    }
    let end = content.trim_end_matches('\n').len();
    content.truncate(end);
}

fn is_math_span(attrs: &[html5ever::Attribute]) -> bool {
    attrs.len() == 1
        && attrs[0].name.local.as_ref() == "class"
        && matches!(
            attrs[0].value.as_ref(),
            "math math-inline" | "math math-display"
        )
}

/// The bytes which, at the start of a text node, could open a CommonMark block
/// — a setext underline, a code fence, a blockquote, a list marker, an ATX
/// heading, or an ordered list number — and so need a leading backslash. Every
/// one is ASCII, so testing the first byte of a text node tests its first
/// character.
fn is_markdown_block_start(byte: u8) -> bool {
    matches!(byte, b'=' | b'~' | b'>' | b'-' | b'+' | b'#' | b'0'..=b'9')
}

/// The bytes which carry CommonMark meaning wherever in a text node they
/// appear, and so are escaped one by one.
fn is_markdown_inline_special(byte: u8) -> bool {
    matches!(byte, b'\\' | b'*' | b'_' | b'`' | b'[' | b']')
}

fn is_plain_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    let Some(&first) = bytes.first() else {
        return true;
    };

    if is_markdown_block_start(first) {
        return false;
    }

    let mut previous_was_space = false;
    for &byte in bytes {
        // `<` opens HTML rather than Markdown, so `escape_if_needed` leaves it
        // to the HTML escape rather than listing it as a special of its own.
        if is_markdown_inline_special(byte) || byte == b'<' {
            return false;
        }
        match byte {
            b' ' => {
                if previous_was_space {
                    return false;
                }
                previous_was_space = true;
            }
            b'\t' | b'\n' | b'\r' | 0x0C | 0x0B => return false,
            _ => previous_was_space = false,
        }
    }

    true
}

pub(crate) fn walk_children(
    node: &Rc<Node>,
    output: &mut String,
    handlers: &ElementHandlers,
    is_parent_block_element: bool,
    // The state the children are walked in.
    state: WalkState,
    // Return value: `markdown_translated`.
) -> bool {
    let mut trim_leading_spaces = !state.is_pre && is_parent_block_element;
    let tag = match &node.data {
        NodeData::Document => Some("html"),
        NodeData::Element { name, .. } => Some(name.local.as_ref()),
        _ => None,
    };
    let mut markdown_translated = true;
    let children = node.children.borrow();
    let mut index = 0;
    while index < children.len() {
        let mut run_end = index + 1;
        while run_end < children.len() && can_combine(&children[run_end - 1], &children[run_end]) {
            run_end += 1;
        }

        let combined;
        let child = if run_end - index > 1 {
            combined = combine_nodes(node, &children[index..run_end]);
            &combined
        } else {
            &children[index]
        };

        let is_block = match &child.data {
            NodeData::Element { name, .. } => is_block_element(&name.local),
            _ => false,
        };

        if is_block {
            // Trim trailing spaces for the previous element
            trim_output_end_spaces(output);
        }

        let output_len = output.len();

        markdown_translated &= walk_node(child, output, handlers, tag, trim_leading_spaces, state);

        if output.len() > output_len {
            // Something was appended, update the flag
            trim_leading_spaces = is_block;
        }

        index = run_end;
    }

    markdown_translated
}

// Determine if the two nodes are similar, and should therefore be combined. If
// so, return the text of the second node to simplify the combining process.
fn can_combine(n1: &Node, n2: &Node) -> bool {
    // To be combined, both nodes must be elements.
    let NodeData::Element {
        name: name1,
        attrs: attrs1,
        template_contents: template_contents1,
        mathml_annotation_xml_integration_point: mathml_annotation_xml_integration_point1,
    } = &n1.data
    else {
        return false;
    };
    let NodeData::Element {
        name: name2,
        attrs: attrs2,
        template_contents: template_contents2,
        mathml_annotation_xml_integration_point: mathml_annotation_xml_integration_point2,
    } = &n2.data
    else {
        return false;
    };

    // Only combine inline content; block content (for example, one paragraph
    // following another) repetition is expected and should not be combined.
    if is_block_element(&name1.local) {
        return false;
    }

    // Their children must be a single text element.
    let c1 = n1.children.borrow();
    let c2 = n2.children.borrow();
    if c1.len() != 1
        || c2.len() != 1
        || !matches!(c1[0].data, NodeData::Text { .. })
        || !matches!(c2[0].data, NodeData::Text { .. })
    {
        return false;
    }

    // Don't combine adjacent hyperlinks.
    *name1.local != *"a"
        && (name1 == name2
            // Treat `i` and `em` tags as the same element; likewise for `b` and
            // `strong`.
            || *name1.local == *"i" && *name2.local == *"em"
            || *name1.local == *"em" && *name2.local == *"i"
            || *name1.local == *"b" && *name2.local == *"strong"
            || *name1.local == *"strong" && name2.local == *"b")
        && template_contents1.borrow().is_none()
        && template_contents2.borrow().is_none()
        && attrs1 == attrs2
        && mathml_annotation_xml_integration_point1 == mathml_annotation_xml_integration_point2
}

fn combine_nodes(parent: &Rc<Node>, nodes: &[Rc<Node>]) -> Rc<Node> {
    let NodeData::Element {
        name,
        attrs,
        template_contents,
        mathml_annotation_xml_integration_point,
    } = &nodes[0].data
    else {
        unreachable!("only compatible element nodes are combined")
    };

    let mut text = Tendril::<UTF8>::new();
    for node in nodes {
        let children = node.children.borrow();
        let NodeData::Text { contents } = &children[0].data else {
            unreachable!("compatible elements have one text child")
        };
        text.push_tendril(&contents.borrow());
    }

    let combined = Node::new(NodeData::Element {
        name: name.clone(),
        attrs: RefCell::new(attrs.borrow().clone()),
        template_contents: RefCell::new(template_contents.borrow().clone()),
        mathml_annotation_xml_integration_point: *mathml_annotation_xml_integration_point,
    });
    combined.parent.set(Some(Rc::downgrade(parent)));

    let text_node = Node::new(NodeData::Text {
        contents: RefCell::new(text),
    });
    text_node.parent.set(Some(Rc::downgrade(&combined)));
    combined.children.borrow_mut().push(text_node);
    combined
}

/// Normalizes content before adding to output by:
/// 1. Collapsing excessive newlines (max 2 consecutive newlines)
/// 2. Collapsing adjacent spaces between inline elements (when not in pre context)
fn append_normalized_content(output: &mut String, mut content: String, is_pre: bool) {
    if output.is_empty() {
        *output = content;
        return;
    }

    // Same newline-count idiom as `join_blocks`: len after trim_matches.
    let last_newlines = output.len() - output.trim_end_matches('\n').len();
    let content_newlines = content.len() - content.trim_start_matches('\n').len();
    let total_newlines = last_newlines + content_newlines;

    // Collapse excessive newlines (max 2)
    if total_newlines > 2 {
        let to_remove = std::cmp::min(total_newlines - 2, content_newlines);
        content.drain(..to_remove);
    }

    // Collapse adjacent spaces between inline elements (not in pre context)
    let content = if !is_pre
        && last_newlines == 0
        && content_newlines == 0
        && output.ends_with(' ')
        && content.starts_with(' ')
    {
        &content[1..]
    } else {
        &content
    };

    output.push_str(content);
}

fn trim_output_end(output: &mut String) {
    let trimmed_len = output.trim_end_document_whitespace().len();
    output.truncate(trimmed_len);
}

fn trim_output_end_spaces(output: &mut String) {
    let trimmed_len = output.trim_end_matches(' ').len();
    output.truncate(trimmed_len);
}

/// Cases:
/// '\'        -> '\\'
/// '==='      -> '\==='      // h1
/// '---'      -> '\---'      // h2
/// '```'      -> '\```'       // code fence
/// '~~~'      -> '\~~~'       // code fence
/// '# Not h1' -> '\\# Not h1' // markdown heading in html
/// '1. Item'  -> '1\\. Item'  // ordered list item
/// '- Item'   -> '\\- Item'   // unordered list item
/// '+ Item'   -> '\\+ Item'   // unordered list item
/// '> Quote'  -> '\\> Quote'  // quote
fn escape_if_needed(text: Cow<'_, str>) -> Cow<'_, str> {
    let Some(first) = text.chars().next() else {
        return text;
    };

    let mut need_escape = is_markdown_block_start(text.as_bytes()[0]);

    if !need_escape {
        // Markdown specials are all ASCII; byte scan avoids UTF-8 decoding.
        need_escape = text.bytes().any(is_markdown_inline_special);
    }

    if !need_escape {
        return crate::html_escape::escape_html(text);
    }

    // Decide structural leading escapes on the raw input before rewriting
    // specials: the specials pass does not alter ATX `#` prefixes or the
    // second-char space of list markers, so pre-escape checks match the old
    // post-escape behavior without `insert(0, ...)`.
    let needs_leading_backslash = match first {
        '=' | '~' | '>' => true,
        '-' | '+' => text.chars().nth(1) == Some(' '),
        '#' => is_markdown_atx_heading(text.as_ref()),
        _ => false,
    };

    let mut escaped = String::with_capacity(text.len() + 8 + usize::from(needs_leading_backslash));
    if needs_leading_backslash {
        escaped.push('\\');
    }
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '*' => escaped.push_str("\\*"),
            '_' => escaped.push_str("\\_"),
            '`' => escaped.push_str("\\`"),
            '[' => escaped.push_str("\\["),
            ']' => escaped.push_str("\\]"),
            _ => escaped.push(ch),
        }
    }

    if first.is_ascii_digit()
        && let Some(dot_idx) = index_of_markdown_ordered_item_dot(&escaped)
    {
        escaped.replace_range(dot_idx..(dot_idx + 1), "\\.");
    }

    // Perform the HTML escape after the other escapes, so that the \\
    // characters inserted here don't get escaped again.
    crate::html_escape::escape_html(escaped.into())
}

/// Cases:
/// '```' -> '\```' // code fence
/// '~~~' -> '\~~~' // code fence
fn escape_pre_text_if_needed(text: Cow<'_, str>) -> Cow<'_, str> {
    let Some(first) = text.chars().next() else {
        return text;
    };
    match first {
        '`' | '~' => {
            let mut escaped = String::with_capacity(text.len() + 1);
            escaped.push('\\');
            escaped.push_str(text.as_ref());
            Cow::Owned(escaped)
        }
        _ => text,
    }
}

pub(crate) static BLOCK_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "address", "article", "aside", "base", "basefont", "blockquote", "body", "caption",
    "center", "col", "colgroup", "dd", "details", "dialog", "dir", "div", "dl", "dt",
    "fieldset", "figcaption", "figure", "footer", "form", "frame", "frameset", "h1", "h2",
    "h3", "h4", "h5", "h6", "head", "header", "hr", "html", "iframe", "legend", "li",
    "link", "main", "menu", "menuitem", "nav", "noframes", "ol", "optgroup", "option", "p",
    "param", "pre", "script", "search", "section", "style", "summary", "table", "tbody", "td",
    "textarea", "tfoot", "th", "thead", "title", "tr", "track", "ul",
};

pub(crate) fn is_block_element(tag: &str) -> bool {
    BLOCK_ELEMENTS.contains(tag)
}
