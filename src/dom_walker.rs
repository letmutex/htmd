use html5ever::{
    Attribute,
    tendril::{Tendril, fmt::UTF8},
};
use markup5ever_rcdom::{Node, NodeData};
use std::{borrow::Cow, cell::RefCell, rc::Rc};

use super::{
    element_handler::ElementHandler,
    node_util::get_node_tag_name,
    options::Options,
    text_util::{
        TrimAsciiWhitespace, compress_whitespace, index_of_markdown_ordered_item_dot,
        is_markdown_atx_heading,
    },
};

pub(crate) fn walk_node(
    node: &Rc<Node>,
    parent_tag: Option<&str>,
    buffer: &mut Vec<String>,
    handler: &dyn ElementHandler,
    options: &Options,
    is_pre: bool,
    trim_leading_spaces: bool,
) {
    match node.data {
        NodeData::Document => {
            walk_children(buffer, node, true, handler, options, false);
            trim_buffer_end(buffer);
        }

        NodeData::Text { ref contents } => {
            append_text(
                buffer,
                parent_tag,
                contents.borrow().to_string(),
                is_pre,
                trim_leading_spaces,
            );
        }

        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            visit_element(
                buffer,
                node,
                handler,
                options,
                &name.local,
                &attrs.borrow(),
                is_pre,
            );
        }

        NodeData::Comment { .. } => {}
        NodeData::Doctype { .. } => {}
        NodeData::ProcessingInstruction { .. } => unreachable!(),
    }
}

fn append_text(
    buffer: &mut Vec<String>,
    parent_tag: Option<&str>,
    text: String,
    is_pre: bool,
    trim_leading_spaces: bool,
) {
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
        let text = ::html_escape::decode_html_entities(&text);
        let text = escape_if_needed(text);
        let text = compress_whitespace(&text);

        let to_add = if trim_leading_spaces
            || (text.chars().next().is_some_and(|ch| ch == ' ')
                && buffer.last().is_some_and(|text| text.ends_with(' ')))
        {
            // We can't compress spaces between two text blocks/elements, so we compress
            // them here by trimming the leading space of current text content.
            text.trim_start_matches(' ').to_string()
        } else {
            text.into_owned()
        };
        buffer.push(to_add);
    }
}

fn visit_element(
    buffer: &mut Vec<String>,
    node: &Rc<Node>,
    handler: &dyn ElementHandler,
    options: &Options,
    tag: &str,
    attrs: &[Attribute],
    is_pre: bool,
) {
    let is_head = tag == "head";
    let is_pre = is_pre || tag == "pre" || tag == "code";
    let prev_buffer_len = buffer.len();
    let is_block = is_block_element(tag);
    walk_children(buffer, node, is_block, handler, options, is_pre);
    let md = handler.on_visit(
        node,
        tag,
        attrs,
        &join_contents(&buffer[prev_buffer_len..]),
        options,
    );
    // Remove the temporary text clips of children
    buffer.truncate(prev_buffer_len);
    if let Some(text) = md
        && (!text.is_empty() || !is_head)
    {
        buffer.push(text);
    }
}

/// Join text clips, inspired by:
/// https://github.com/mixmark-io/turndown/blob/cc73387fb707e5fb5e1083e94078d08f38f3abc8/src/turndown.js#L221
fn join_contents(contents: &[String]) -> String {
    let mut result = String::new();
    for content in contents {
        let content_len = content.len();
        if content_len == 0 {
            continue;
        }

        let result_len = result.len();

        let left = result.trim_end_matches('\n');
        let right = content.trim_start_matches('\n');

        let max_trimmed_new_lines =
            std::cmp::max(result_len - left.len(), content_len - right.len());
        let separator_new_lines = std::cmp::min(max_trimmed_new_lines, 2);
        let separator = "\n".repeat(separator_new_lines);

        let mut next_result = String::with_capacity(left.len() + separator.len() + right.len());
        next_result.push_str(left);
        next_result.push_str(&separator);
        next_result.push_str(right);

        result = next_result;
    }
    result
}

// Determine if the two nodes are similar, and should therefore be combined. If so, return the text of the second node to simplify the combining process.
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

    // Only combine inline content; block content (for example, one paragraph following another) repetition is expected and should not be combined.
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
        && (name1 == name2
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

fn walk_children(
    buffer: &mut Vec<String>,
    node: &Rc<Node>,
    is_parent_block_element: bool,
    handler: &dyn ElementHandler,
    options: &Options,
    is_pre: bool,
) {
    // Combine similar adjacent blocks.
    let mut children = node.children.borrow_mut();
    let mut index = 1;
    while index < children.len() {
        if let Some(text) = can_combine(&children[index - 1], &children[index]) {
            // Combine the text from `chidren[index]` with `children[index - 1]`, then remove `children[index]`.
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

    // This will trim leading spaces of the first element/text in block
    // elements (except pre and code elements)
    let mut trim_leading_spaces = !is_pre && is_parent_block_element;
    let tag = get_node_tag_name(node);
    for child in node.children.borrow().iter() {
        let is_block = get_node_tag_name(child).is_some_and(is_block_element);

        if is_block {
            // Trim trailing spaces for the previous element
            trim_buffer_end_spaces(buffer);
        }

        let buffer_len = buffer.len();

        walk_node(
            child,
            tag,
            buffer,
            handler,
            options,
            is_pre,
            trim_leading_spaces,
        );

        if buffer.len() > buffer_len {
            // Something was appended, update the flag
            trim_leading_spaces = is_block;
        }
    }
}

fn trim_buffer_end(buffer: &mut [String]) {
    for content in buffer.iter_mut().rev() {
        let trimmed = content.trim_end_ascii_whitespace();
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

    // Perform the HTML escape after the other escapes, so that the \ characters inserted here don't get escaped again.
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

fn is_block_container(tag: &str) -> bool {
    matches!(
        tag,
        "html"
            | "body"
            | "div"
            | "ul"
            | "ol"
            | "li"
            | "table"
            | "tr"
            | "header"
            | "head"
            | "footer"
            | "nav"
            | "section"
            | "article"
            | "aside"
            | "main"
            | "blockquote"
            | "script"
            | "style"
    )
}

fn is_block_element(tag: &str) -> bool {
    if is_block_container(tag) {
        return true;
    }
    matches!(
        tag,
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "pre" | "hr" | "br"
    )
}
