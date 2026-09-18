use html5ever::tendril::{Tendril, fmt::UTF8};
use markup5ever_rcdom::{Node, NodeData};
use std::{borrow::Cow, cell::RefCell, rc::Rc};

use crate::{
    Context, Element,
    element_handler::{ElementHandlers, element_util::serialize_element},
};

use super::{
    options::TranslationMode,
    util::{
        escape::{escape_html, index_of_markdown_ordered_item_delimiter, is_markdown_atx_heading},
        node::is_block_element,
        text::{TrimDocumentWhitespace, compress_whitespace, concat_strings, frame_as_block},
    },
};

pub(crate) fn walk_node(
    node: &Rc<Node>,
    output: &mut String,
    handlers: &ElementHandlers,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    context: Context,
) -> bool {
    match &node.data {
        NodeData::Document => {
            let _ = walk_children(node, output, handlers, true, Context::BLOCK);
            trim_output_end(output);
            true
        }
        NodeData::Text { contents } => walk_text(
            contents.borrow().as_ref(),
            output,
            parent_tag,
            trim_leading_spaces,
            context,
        ),
        NodeData::Element { name, attrs, .. } => walk_element(
            node,
            name.local.as_ref(),
            &attrs.borrow(),
            output,
            handlers,
            context,
        ),
        NodeData::Comment { contents } => {
            if handlers.options.translation_mode == TranslationMode::Faithful {
                walk_comment(contents, output, context);
            }
            true
        }
        NodeData::Doctype { .. } => true,
        NodeData::ProcessingInstruction { .. } => true,
    }
}

/// A comment is an
/// [HTML block](https://spec.commonmark.org/0.31.2/#html-blocks) of type 2,
/// which ends at its own `-->` rather than at a blank line: in a block context
/// it is framed as a block and its line endings are left as they are.
///
/// In an inline context it is a raw HTML inline, whose whitespace the
/// "Translating HTML nodes" section of `unsupported_html.md` collapses.
/// Encoding the line endings instead would be no use: no parser decodes a
/// character reference inside a comment.
fn walk_comment(contents: &str, output: &mut String, context: Context) {
    let html = concat_strings!("<!--", contents, "-->");
    if context.is_block() {
        append_normalized_content(output, frame_as_block(&html), context.literal);
    } else {
        output.push_str(&compress_whitespace(&html));
    }
}

fn walk_text(
    text: &str,
    output: &mut String,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    context: Context,
) -> bool {
    if context.literal {
        let text = if parent_tag == Some("pre") {
            escape_pre_text_if_needed(Cow::Borrowed(text))
        } else {
            Cow::Borrowed(text)
        };
        output.push_str(text.as_ref());
        return true;
    }

    // Escape last, so that the checks for the whitespace which closes a marker
    // read the text the output will hold: a line ending or a tab there is a
    // space once the whitespace is compressed, and `trim_leading_spaces`
    // removes the spaces which would otherwise hide a marker from those
    // checks.
    //
    // Without that flag the leading space survives, so `escape_if_needed`
    // reads it rather than the marker behind it. That is right mid-line, which
    // is what a clear flag usually means, but wrong after an inline construct
    // which ends the line -- `<br>` outside faithful mode -- where such a
    // marker is left unescaped.
    let text = compress_whitespace(text);
    let text = if trim_leading_spaces {
        text.trim_start_matches(' ')
    } else {
        text.as_ref()
    };
    let text = escape_if_needed(Cow::Borrowed(text));
    let text = if !trim_leading_spaces && output.ends_with(' ') && text.starts_with(' ') {
        &text[1..]
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
    context: Context,
) -> bool {
    if is_passthrough_span(tag, attrs, handlers) {
        let mut content = String::new();
        // A span holds inline content, so its children keep this context.
        let markdown_translated = walk_children(node, &mut content, handlers, false, context);
        trim_newlines(&mut content);
        append_normalized_content(output, content, context.literal);
        return markdown_translated;
    }

    if handlers.options.translation_mode == TranslationMode::Pure
        && !handlers.has_tag_handler_or_listeners(tag)
    {
        let mut content = String::new();
        let is_block = is_block_element(tag);
        let markdown_translated = walk_children(node, &mut content, handlers, is_block, context);
        append_normalized_content(output, content, context.literal);
        return markdown_translated;
    }

    let Some(result) = handlers.handle(node, tag, attrs, 0, context) else {
        return true;
    };
    if !result.content.is_empty() || tag != "head" {
        append_normalized_content(output, result.content, context.literal);
    }
    result.markdown_translated
}

fn is_passthrough_span(
    tag: &str,
    attrs: &[html5ever::Attribute],
    handlers: &ElementHandlers,
) -> bool {
    tag == "span" && handlers.can_passthrough_span && !is_math_span(attrs)
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
/// heading, or an ordered list number. Every one is ASCII, so testing the first
/// byte of a text node tests its first character.
///
/// Could, not does: only `=`, `~` and `>` open their block on their own. The
/// rest need what follows them as well, which [`escape_if_needed`] decides.
/// This is the cheap test which sends a text node there at all.
fn is_markdown_block_start(byte: u8) -> bool {
    matches!(byte, b'=' | b'~' | b'>' | b'-' | b'+' | b'#' | b'0'..=b'9')
}

/// The bytes which carry CommonMark meaning wherever in a text node they
/// appear, and so are escaped one by one.
fn is_markdown_inline_special(byte: u8) -> bool {
    matches!(byte, b'\\' | b'*' | b'_' | b'`' | b'[' | b']')
}

pub(crate) fn walk_children(
    node: &Rc<Node>,
    output: &mut String,
    handlers: &ElementHandlers,
    is_parent_block_element: bool,
    // The context the children are walked in.
    context: Context,
    // Return value: `markdown_translated`.
) -> bool {
    let mut trim_leading_spaces = !context.literal && is_parent_block_element;
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
        let child = if run_end - index == 1 {
            &children[index]
        } else if handlers.options.translation_mode == TranslationMode::Pure {
            combined = combine_nodes(node, &children[index..run_end]);
            &combined
        } else {
            // Combining writes a run of elements as one, which faithful mode
            // owes the reader as the elements they were. Writing them one after
            // the other is no answer either, since that is what combining
            // avoids: `*a**b*` is a single emphasis holding `a**b`, and
            // `` `a``b` `` a single code span. Each element goes out as HTML.
            for element_node in &children[index..run_end] {
                append_serialized_inline(element_node, output, handlers, context);
            }
            markdown_translated = false;
            trim_leading_spaces = false;
            index = run_end;
            continue;
        };

        let is_block = match &child.data {
            NodeData::Element { name, .. } => is_block_element(&name.local),
            _ => false,
        };

        // Preformatted content keeps every space it holds, so a block inside
        // one trims nothing.
        let trims_spaces = is_block && !context.literal;

        if trims_spaces {
            // Trim trailing spaces for the previous element
            trim_output_end_spaces(output);
        }

        let output_len = output.len();

        markdown_translated &=
            walk_node(child, output, handlers, tag, trim_leading_spaces, context);

        if output.len() > output_len {
            // Something was appended, update the flag
            trim_leading_spaces = trims_spaces;
        }

        index = run_end;
    }

    markdown_translated
}

// Determine if the two nodes are similar enough that writing them one after the
// other would read back as a single element. Pure mode combines such a run into
// the one element the CommonMark spells; faithful mode writes each of them as
// HTML instead.
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

/// Appends `node`, an element of a combinable run, to `output` as a raw HTML
/// inline. Only inline elements form a run, so the serializer's block spelling
/// never applies here.
fn append_serialized_inline(
    node: &Rc<Node>,
    output: &mut String,
    handlers: &ElementHandlers,
    context: Context,
) {
    let NodeData::Element { name, attrs, .. } = &node.data else {
        unreachable!("only element nodes form a combinable run")
    };
    let element = Element {
        node,
        tag: name.local.as_ref(),
        attrs: &attrs.borrow(),
        context,
        skipped_handlers: 0,
    };
    let html = serialize_element(handlers, &element);
    append_normalized_content(output, html, context.literal);
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
/// 2. Collapsing adjacent spaces between inline elements
fn append_normalized_content(output: &mut String, mut content: String, preserve_whitespace: bool) {
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

    // Collapse adjacent spaces between inline elements
    let content = if !preserve_whitespace
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
///
/// ````text
/// '\'        -> '\\'
/// '==='      -> '\==='      // setext underline
/// '---'      -> '\---'      // setext underline, thematic break
/// '```'      -> '\`\`\`'    // code fence, escaped as three inline specials
/// '~~~'      -> '\~~~'      // code fence
/// '# Not h1' -> '\# Not h1' // ATX heading
/// '1. Item'  -> '1\. Item'  // ordered list item
/// '1) Item'  -> '1\) Item'  // ordered list item
/// '- Item'   -> '\- Item'   // bullet list item
/// '+ Item'   -> '\+ Item'   // bullet list item
/// '> Quote'  -> '\> Quote'  // block quote
/// ````
///
/// A marker needs no text after it: whitespace closes it, and so does the end
/// of its line. The cases which look for that terminator -- `#`, `-`, `+`, and
/// an ordered item's `.` or `)` -- therefore read the end of `text` as the end
/// of the line, which `text` cannot tell them. Where an inline sibling
/// continues the line instead -- `<p>a <em>b</em>#</p>` -- the marker opens
/// nothing and is escaped all the same. The backslash reads back as the text it
/// came from, so the cost is a character, not a meaning.
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
        return escape_html(text);
    }

    // Decide structural leading escapes on the raw input before rewriting
    // specials: the specials pass does not alter ATX `#` prefixes or the
    // second-char space of list markers, so pre-escape checks match the old
    // post-escape behavior without `insert(0, ...)`.
    let needs_leading_backslash = match first {
        '=' | '~' | '>' => true,
        // A line of nothing but `-` and spaces is also a thematic break or a
        // setext underline. Anything else after the second `-` -- `--foo` --
        // is neither, and opens no list either.
        '-' => {
            matches!(text.as_bytes().get(1), Some(b' '))
                || text.bytes().all(|byte| matches!(byte, b'-' | b' '))
        }
        '+' => matches!(text.as_bytes().get(1), None | Some(b' ')),
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
        && let Some(delimiter_idx) = index_of_markdown_ordered_item_delimiter(&escaped)
    {
        escaped.insert(delimiter_idx, '\\');
    }

    // Perform the HTML escape after the other escapes, so that the \\
    // characters inserted here don't get escaped again.
    escape_html(escaped.into())
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
