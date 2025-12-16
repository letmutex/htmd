use html5ever::tendril::{Tendril, fmt::UTF8};
use markup5ever_rcdom::{Node, NodeData};
use phf::phf_set;
use std::{borrow::Cow, cell::RefCell, rc::Rc};

use crate::element_handler::ElementHandlers;

use super::{
    node_util::get_node_tag_name,
    options::TranslationMode,
    text_util::{
        TrimDocumentWhitespace, compress_whitespace, index_of_markdown_ordered_item_dot,
        is_markdown_atx_heading,
    },
};

pub(crate) fn walk_node(
    node: &Rc<Node>,
    buffer: &mut Vec<String>,
    handlers: &ElementHandlers,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    is_pre: bool,
) -> bool {
    let mut markdown_translated = true;
    match node.data {
        NodeData::Document => {
            let _ = walk_children(node, buffer, handlers, true, false);
            trim_buffer_end(buffer);
        }

        NodeData::Text { ref contents } => {
            // Append the text in this node to the buffer.
            let text = contents.borrow().to_string();
            if is_pre {
                // Handle pre and code
                let text = if parent_tag.is_some_and(|t| t == "pre") {
                    escape_pre_text_if_needed(text)
                } else {
                    text
                };
                buffer.push(text);
            } else {
                // Handle other elements or texts
                let text = escape_if_needed(Cow::Owned(text));
                let text = compress_whitespace(text.as_ref());

                let to_add = if trim_leading_spaces
                    || (text.chars().next().is_some_and(|ch| ch == ' ')
                        && buffer.last().is_some_and(|text| text.ends_with(' ')))
                {
                    // We can't compress spaces between two text blocks/elements, so we
                    // compress them here by trimming the leading space of current text
                    // content.
                    text.trim_start_matches(' ').to_string()
                } else {
                    text.into_owned()
                };
                if !to_add.is_empty() {
                    buffer.push(to_add);
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

            let res = handlers.handle(
                node,
                tag,
                &attrs.borrow(),
                true, // Default to true, handler will update
                0,
            );

            if let Some(res) = res {
                markdown_translated = res.markdown_translated;
                if !res.content.is_empty() || !is_head {
                    let content = normalize_content_for_buffer(buffer.last(), res.content, is_pre);
                    if !content.is_empty() {
                        buffer.push(content);
                    }
                }
            }
        }

        NodeData::Comment { ref contents } => {
            if handlers.options.translation_mode == TranslationMode::Faithful {
                buffer.push(format!("<!--{}-->", contents));
            }
        }
        NodeData::Doctype { .. } => {}
        NodeData::ProcessingInstruction { .. } => unreachable!(),
    }

    markdown_translated
}

pub(crate) fn walk_children(
    node: &Rc<Node>,
    buffer: &mut Vec<String>,
    handlers: &ElementHandlers,
    is_parent_block_element: bool,
    is_pre: bool,
    // Return value: `markdown_translated`.
) -> bool {
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
    drop(children);

    // Trim leading spaces of the first element/text in block elements (except pre/code)
    let mut trim_leading_spaces = !is_pre && is_parent_block_element;
    let tag = get_node_tag_name(node);
    let mut markdown_translated = true;
    for child in node.children.borrow().iter() {
        let is_block = get_node_tag_name(child).is_some_and(is_block_element);

        if is_block {
            // Trim trailing spaces for the previous element
            trim_buffer_end_spaces(buffer);
        }

        let buffer_len = buffer.len();

        markdown_translated &= walk_node(child, buffer, handlers, tag, trim_leading_spaces, is_pre);

        if buffer.len() > buffer_len {
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

/// Normalizes content before adding to buffer by:
/// 1. Collapsing excessive newlines (max 2 consecutive newlines)
/// 2. Collapsing adjacent spaces between inline elements (when not in pre context)
fn normalize_content_for_buffer(
    last_buffer_item: Option<&String>,
    mut content: String,
    is_pre: bool,
) -> String {
    let Some(last) = last_buffer_item else {
        return content;
    };

    let last_newlines = last.chars().rev().take_while(|c| *c == '\n').count();
    let content_newlines = content.chars().take_while(|c| *c == '\n').count();
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
        && last.chars().last().is_some_and(|c| c == ' ')
        && content.chars().next().is_some_and(|c| c == ' ')
    {
        content.remove(0);
    }

    content
}

fn trim_buffer_end(buffer: &mut [String]) {
    for content in buffer.iter_mut().rev() {
        let trimmed = content.trim_end_document_whitespace();
        if trimmed.len() == content.len() {
            break;
        }
        *content = trimmed.to_string();
    }
}

fn trim_buffer_end_spaces(buffer: &mut [String]) {
    for content in buffer.iter_mut().rev() {
        let trimmed = content.trim_end_matches(' ');
        if trimmed.len() == content.len() {
            break;
        }
        *content = trimmed.to_string();
    }
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
        need_escape = text
            .chars()
            .any(|c| c == '\\' || c == '*' || c == '_' || c == '`' || c == '[' || c == ']');
    }

    if !need_escape {
        return crate::html_escape::escape_html(text);
    }

    let mut escaped = String::new();
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

    match first {
        '=' | '~' | '>' => {
            escaped.insert(0, '\\');
        }
        '-' | '+' => {
            if escaped.chars().nth(1).is_some_and(|ch| ch == ' ') {
                escaped.insert(0, '\\');
            }
        }
        '#' => {
            if is_markdown_atx_heading(&escaped) {
                escaped.insert(0, '\\');
            }
        }
        '0'..='9' => {
            if let Some(dot_idx) = index_of_markdown_ordered_item_dot(&escaped) {
                escaped.replace_range(dot_idx..(dot_idx + 1), "\\.");
            }
        }
        _ => {}
    }

    // Perform the HTML escape after the other escapes, so that the \\
    // characters inserted here don't get escaped again.
    crate::html_escape::escape_html(escaped.into())
}

/// Cases:
/// '```' -> '\```' // code fence
/// '~~~' -> '\~~~' // code fence
fn escape_pre_text_if_needed(text: String) -> String {
    let Some(first) = text.chars().next() else {
        return text;
    };
    match first {
        '`' | '~' => {
            let mut text = text;
            text.insert(0, '\\');
            text
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
