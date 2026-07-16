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

pub(crate) fn walk_node(
    node: &Rc<Node>,
    output: &mut String,
    handlers: &ElementHandlers,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    is_pre: bool,
) -> bool {
    let mut markdown_translated = true;
    match node.data {
        NodeData::Document => {
            let _ = walk_children(node, output, handlers, true, false);
            trim_output_end(output);
        }

        NodeData::Text { ref contents } => {
            let text = contents.borrow();
            let text = text.as_ref();
            if is_pre {
                // Handle pre and code
                let text = if parent_tag.is_some_and(|t| t == "pre") {
                    escape_pre_text_if_needed(Cow::Borrowed(text))
                } else {
                    Cow::Borrowed(text)
                };
                output.push_str(text.as_ref());
            } else {
                let last_ends_with_space = output.ends_with(' ');
                if is_plain_text(text) {
                    let text =
                        if trim_leading_spaces || (text.starts_with(' ') && last_ends_with_space) {
                            text.trim_start_matches(' ')
                        } else {
                            text
                        };
                    if !text.is_empty() {
                        output.push_str(text);
                    }
                    return markdown_translated;
                }

                // Handle other elements or texts
                let text = escape_if_needed(Cow::Borrowed(text));
                let text = compress_whitespace(text.as_ref());

                let to_add = if trim_leading_spaces
                    || (text.chars().next().is_some_and(|ch| ch == ' ') && last_ends_with_space)
                {
                    // We can't compress spaces between two text blocks/elements, so we
                    // compress them here by trimming the leading space of current text
                    // content.
                    text.trim_start_matches(' ')
                } else {
                    text.as_ref()
                };
                if !to_add.is_empty() {
                    output.push_str(to_add);
                }
            }
        }

        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            // Visit this element.
            let tag = &*name.local;
            let is_head = tag == "head";
            let attrs = attrs.borrow();

            if tag == "span"
                && handlers.options.translation_mode == TranslationMode::Pure
                && handlers
                    .tag_to_handler_indices
                    .get("span")
                    .is_some_and(|indices| indices.len() == 1)
                && !is_math_span(&attrs)
            {
                let mut content = String::new();
                let is_pre =
                    is_pre || (parent_tag.is_none() && crate::element_handler::is_inside_pre(node));
                let markdown_translated =
                    walk_children(node, &mut content, handlers, false, is_pre);

                let start = content.len() - content.trim_start_matches('\n').len();
                if start > 0 {
                    content.drain(..start);
                }
                let end = content.trim_end_matches('\n').len();
                content.truncate(end);

                append_normalized_content(output, content, is_pre);
                return markdown_translated;
            }

            if handlers.options.translation_mode == TranslationMode::Pure
                && !handlers.tag_to_handler_indices.contains_key(tag)
            {
                let mut content = String::new();
                let is_pre =
                    is_pre || (parent_tag.is_none() && crate::element_handler::is_inside_pre(node));
                let markdown_translated =
                    walk_children(node, &mut content, handlers, is_block_element(tag), is_pre);
                append_normalized_content(output, content, is_pre);
                return markdown_translated;
            }

            let res = handlers.handle(node, tag, &attrs, true, 0);

            if let Some(res) = res {
                markdown_translated = res.markdown_translated;
                if !res.content.is_empty() || !is_head {
                    append_normalized_content(output, res.content, is_pre);
                }
            }
        }

        NodeData::Comment { ref contents } => {
            if handlers.options.translation_mode == TranslationMode::Faithful {
                output.push_str("<!--");
                output.push_str(contents);
                output.push_str("-->");
            }
        }
        NodeData::Doctype { .. } => {}
        NodeData::ProcessingInstruction { .. } => unreachable!(),
    }

    markdown_translated
}

fn is_math_span(attrs: &[html5ever::Attribute]) -> bool {
    attrs.len() == 1
        && attrs[0].name.local.as_ref() == "class"
        && matches!(
            attrs[0].value.as_ref(),
            "math math-inline" | "math math-display"
        )
}

fn is_plain_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    let Some(&first) = bytes.first() else {
        return true;
    };

    if matches!(first, b'=' | b'~' | b'>' | b'-' | b'+' | b'#' | b'0'..=b'9') {
        return false;
    }

    let mut previous_was_space = false;
    for &byte in bytes {
        match byte {
            b'\\' | b'*' | b'_' | b'`' | b'[' | b']' | b'<' => return false,
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
    is_pre: bool,
    // Return value: `markdown_translated`.
) -> bool {
    if node.children.borrow().len() > 1 {
        // Combine similar adjacent blocks.
        let mut children = node.children.borrow_mut();
        let mut index = 1;
        while index < children.len() {
            if let Some(text) = can_combine(&children[index - 1], &children[index]) {
                // Combine the text from `chidren[index]` with `children[index -
                // 1]`, then remove `children[index]`.
                children.remove(index);
                index -= 1;
                let children_of_index = children.get(index).unwrap().children.borrow();
                let text_data = &children_of_index.first().unwrap().data;
                let NodeData::Text { contents } = text_data else {
                    panic!("")
                };
                let mut inner_contents = contents.clone().into_inner();
                inner_contents.push_tendril(&text.take());
                contents.replace(inner_contents);
            }
            index += 1;
        }
    }

    // Trim leading spaces of the first element/text in block elements (except pre/code)
    let mut trim_leading_spaces = !is_pre && is_parent_block_element;
    let tag = match &node.data {
        NodeData::Document => Some("html"),
        NodeData::Element { name, .. } => Some(name.local.as_ref()),
        _ => None,
    };
    let mut markdown_translated = true;
    for child in node.children.borrow().iter() {
        let is_block = match &child.data {
            NodeData::Element { name, .. } => is_block_element(&name.local),
            _ => false,
        };

        if is_block {
            // Trim trailing spaces for the previous element
            trim_output_end_spaces(output);
        }

        let output_len = output.len();

        markdown_translated &= walk_node(child, output, handlers, tag, trim_leading_spaces, is_pre);

        if output.len() > output_len {
            // Something was appended, update the flag
            trim_leading_spaces = is_block;
        }
    }

    markdown_translated
}

// Determine if the two nodes are similar, and should therefore be combined. If
// so, return the text of the second node to simplify the combining process.
fn can_combine(n1: &Node, n2: &Node) -> Option<RefCell<Tendril<UTF8>>> {
    // To be combined, both nodes must be elements.
    let NodeData::Element {
        name: name1,
        attrs: attrs1,
        template_contents: template_contents1,
        mathml_annotation_xml_integration_point: mathml_annotation_xml_integration_point1,
    } = &n1.data
    else {
        return None;
    };
    let NodeData::Element {
        name: name2,
        attrs: attrs2,
        template_contents: template_contents2,
        mathml_annotation_xml_integration_point: mathml_annotation_xml_integration_point2,
    } = &n2.data
    else {
        return None;
    };

    // Only combine inline content; block content (for example, one paragraph
    // following another) repetition is expected and should not be combined.
    if is_block_element(&name1.local) {
        return None;
    }

    // Their children must be a single text element.
    let c1 = n1.children.borrow();
    let c2 = n2.children.borrow();
    if c1.len() == 1
        && c2.len() == 1
        && let Some(d1) = c1.first()
        && let Some(d2) = c2.first()
        && let NodeData::Text {
            contents: _contents1,
        } = &d1.data
        && let NodeData::Text {
            contents: contents2,
        } = &d2.data
        // Don't combine adjacent hyperlinks.
        && *name1.local != *"a"
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
    {
        Some(contents2.clone())
    } else {
        None
    }
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
    if !is_pre
        && last_newlines == 0
        && content_newlines == 0
        && output.ends_with(' ')
        && content.starts_with(' ')
    {
        content.remove(0);
    }

    output.push_str(&content);
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

    let mut need_escape = matches!(first, '=' | '~' | '>' | '-' | '+' | '#' | '0'..='9');

    if !need_escape {
        // Markdown specials are all ASCII; byte scan avoids UTF-8 decoding.
        need_escape = text
            .bytes()
            .any(|b| matches!(b, b'\\' | b'*' | b'_' | b'`' | b'[' | b']'));
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

// This is taken from the
// [CommonMark spec](https://spec.commonmark.org/0.31.2/#html-blocks).
static BLOCK_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "pre",
    "script",
    "search",
    "section",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
};

pub(crate) fn is_block_element(tag: &str) -> bool {
    BLOCK_ELEMENTS.contains(tag)
}
