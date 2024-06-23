use html5ever::Attribute;
use markup5ever_rcdom::{Node, NodeData};
use std::rc::Rc;

use super::{
    element_handler::ElementHandler,
    node_util::get_node_tag_name,
    options::Options,
    text_util::{
        compress_whitespace, index_of_markdown_ordered_item_dot, is_markdown_atx_heading,
        TrimAsciiWhitespace,
    },
};

pub(crate) fn walk_node(
    node: &Rc<Node>,
    parent_tag: Option<&String>,
    buffer: &mut Vec<String>,
    handler: &Box<&dyn ElementHandler>,
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
            let attrs_value = attrs.take();
            // Put it back
            attrs.replace(attrs_value.clone());
            visit_element(
                buffer,
                node,
                handler,
                options,
                name.local.to_string(),
                attrs_value,
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
    parent_tag: Option<&String>,
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
        let text = escape_if_needed(text);
        let text = compress_whitespace(&text);

        let mut chars = text.chars();
        if chars.next().is_some_and(|ch| ch == ' ')
            && chars.next().is_none()
            && parent_tag.is_some_and(|tag| is_block_container(tag))
        {
            // Ignore whitespace in block containers.
            return;
        }

        let to_add = if trim_leading_spaces
            || (text.chars().nth(0).is_some_and(|ch| ch == ' ')
                && buffer.last().is_some_and(|text| text.ends_with(" ")))
        {
            // We can't compress spaces between two text blocks/elements, so we compress
            // them here by trimming the leading space of current text content.
            text.trim_start_matches(|ch| ch == ' ').to_string()
        } else {
            text
        };
        buffer.push(to_add);
    }
}

fn visit_element(
    buffer: &mut Vec<String>,
    node: &Rc<Node>,
    handler: &Box<&dyn ElementHandler>,
    options: &Options,
    tag: String,
    attrs: Vec<Attribute>,
    is_pre: bool,
) {
    let is_head = tag == "head";
    let is_pre = is_pre || tag == "pre" || tag == "code";
    let prev_buffer_len = buffer.len();
    let is_block = is_block_element(&tag);
    walk_children(buffer, node, is_block, handler, options, is_pre);
    let md = handler.on_visit(
        node.clone(),
        tag,
        attrs,
        join_contents(&buffer[prev_buffer_len..]),
        options,
    );
    // Remove the temporary text clips of children
    buffer.truncate(prev_buffer_len);
    if let Some(text) = md {
        if !text.is_empty() || !is_head {
            buffer.push(text);
        }
    }
}

/// Join text clips, inspired by:
/// https://github.com/mixmark-io/turndown/blob/cc73387fb707e5fb5e1083e94078d08f38f3abc8/src/turndown.js#L221
fn join_contents(contents: &[String]) -> String {
    let mut result = String::new();
    for content in contents {
        let content_len = content.chars().count();
        if content_len == 0 {
            continue;
        }

        let result_len = result.chars().count();

        let left = result.trim_end_matches(|ch| ch == '\n');
        let right = content.trim_start_matches(|ch| ch == '\n');

        let max_trimmed_new_lines = std::cmp::max(
            result_len - left.chars().count(),
            content_len - right.chars().count(),
        );
        let separator_new_lines = std::cmp::min(max_trimmed_new_lines, 2);
        let separator = "\n".repeat(separator_new_lines);

        let mut next_result = String::with_capacity(left.len() + separator.len() + right.len());
        next_result.push_str(&left);
        next_result.push_str(&separator);
        next_result.push_str(right);

        result = next_result;
    }
    result
}

fn walk_children(
    buffer: &mut Vec<String>,
    node: &Rc<Node>,
    is_parent_blok_element: bool,
    handler: &Box<&dyn ElementHandler>,
    options: &Options,
    is_pre: bool,
) {
    let tag = get_node_tag_name(node);
    // This will trim leading spaces of the first element/text in block
    // elements (except pre and code elements)
    let mut trim_leading_spaces = !is_pre && is_parent_blok_element;
    for child in node.children.borrow().iter() {
        let is_block = get_node_tag_name(child).is_some_and(|tag| is_block_element(&tag));

        if is_block {
            // Trim trailing spaces for the previous element
            trim_buffer_end_spaces(buffer);
        }

        let buffer_len = buffer.len();

        walk_node(
            child,
            tag.as_ref(),
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

fn trim_buffer_end(buffer: &mut Vec<String>) {
    for content in buffer.iter_mut().rev() {
        let trimmed = content.trim_end_ascii_whitespace();
        if trimmed.len() == content.len() {
            break;
        }
        *content = trimmed.to_string();
    }
}

fn trim_buffer_end_spaces(buffer: &mut Vec<String>) {
    for content in buffer.iter_mut().rev() {
        let trimmed = content.trim_end_matches(|ch| ch == ' ');
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
fn escape_if_needed(text: String) -> String {
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
    let Some(first) = escaped.chars().nth(0) else {
        return escaped;
    };
    match first {
        '=' | '~' | '>' => {
            escaped.insert(0, '\\');
            escaped
        }
        '-' | '+' => {
            if escaped.chars().nth(1).is_some_and(|ch| ch == ' ') {
                escaped.insert(0, '\\');
            }
            escaped
        }
        '#' => {
            if is_markdown_atx_heading(&escaped) {
                escaped.insert(0, '\\');
            }
            escaped
        }
        '0'..='9' => {
            if let Some(dot_idx) = index_of_markdown_ordered_item_dot(&escaped) {
                escaped.replace_range(dot_idx..(dot_idx + 1), "\\.");
            }
            escaped
        }
        _ => escaped,
    }
}

/// Cases:
/// '```' -> '\```' // code fence
/// '~~~' -> '\~~~' // code fence
fn escape_pre_text_if_needed(text: String) -> String {
    let Some(first) = text.chars().nth(0) else {
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
    match tag {
        "html" | "body" | "div" | "ul" | "ol" | "li" | "table" | "tr" | "header" | "head"
        | "footer" | "nav" | "section" | "article" | "aside" | "main" | "blockquote" | "script"
        | "style" => true,
        _ => false,
    }
}

fn is_block_element(tag: &str) -> bool {
    if is_block_container(tag) {
        return true;
    }
    match tag {
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "pre" | "hr" | "br" => true,
        _ => false,
    }
}
