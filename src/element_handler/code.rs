use std::rc::Rc;

use html5ever::Attribute;
use markup5ever_rcdom::{Node, NodeData};

use crate::{
    Element,
    element_handler::{HandlerResult, Handlers, serialize_element},
    node_util::{get_node_tag_name, get_parent_node},
    options::{CodeBlockFence, CodeBlockStyle, TranslationMode},
    serialize_if_faithful,
    text_util::{JoinOnStringIterator, TrimDocumentWhitespace, concat_strings},
};

pub(super) fn code_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // In faithful mode, all children of a code tag must be text to translate
    // as markdown.
    if handlers.options().translation_mode == TranslationMode::Faithful
        && !element
            .node
            .children
            .borrow()
            .iter()
            .all(|node| matches!(node.data, NodeData::Text { .. }))
    {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }

    // Determine the type: inline code or a code block.
    let parent_node = get_parent_node(element.node);
    let is_code_block = parent_node
        .as_ref()
        .map(|parent| get_node_tag_name(parent).is_some_and(|t| t == "pre"))
        .unwrap_or(false);
    if is_code_block {
        handle_code_block(handlers, element, &parent_node.unwrap())
    } else {
        handle_inline_code(handlers, element)
    }
}

fn handle_code_block(
    handlers: &dyn Handlers,
    element: Element,
    parent: &Rc<Node>,
) -> Option<HandlerResult> {
    let content = handlers.walk_children(element.node).content;
    let content = content.strip_suffix('\n').unwrap_or(&content);
    if handlers.options().code_block_style == CodeBlockStyle::Fenced {
        let fence = if handlers.options().code_block_fence == CodeBlockFence::Tildes {
            get_code_fence_marker("~", content)
        } else {
            get_code_fence_marker("`", content)
        };
        let language = find_language_from_attrs(element.attrs).or_else(|| {
            if let NodeData::Element { ref attrs, .. } = parent.data {
                find_language_from_attrs(&attrs.borrow())
            } else {
                None
            }
        });
        serialize_if_faithful!(handlers, element, if language.is_none() { 0 } else { 1 });
        let mut result = String::from(&fence);
        if let Some(ref lang) = language {
            result.push_str(lang);
        }
        result.push('\n');
        result.push_str(content);
        result.push('\n');
        result.push_str(&fence);
        Some(result.into())
    } else {
        serialize_if_faithful!(handlers, element, 0);
        let code = content
            .lines()
            .map(|line| concat_strings!("    ", line))
            .join("\n");
        Some(code.into())
    }
}

fn get_code_fence_marker(symbol: &str, content: &str) -> String {
    let symbol = symbol
        .chars()
        .next()
        .expect("code fence symbol cannot be empty");
    let mut longest_run = 0;
    let mut current_run = 0;

    for ch in content.chars() {
        if ch == symbol {
            current_run += 1;
            longest_run = longest_run.max(current_run);
        } else {
            current_run = 0;
        }
    }

    symbol.to_string().repeat(3.max(longest_run + 1))
}

fn find_language_from_attrs(attrs: &[Attribute]) -> Option<String> {
    attrs
        .iter()
        .find(|attr| &attr.name.local == "class")
        .map(|attr| {
            attr.value
                .split(' ')
                .find(|cls| cls.starts_with("language-"))
                .map(|lang| lang.split('-').skip(1).join("-"))
        })
        .unwrap_or(None)
}

fn handle_inline_code(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let content = handlers.walk_children(element.node).content;
    let preserve_boundary_spaces = handlers.options().preformatted_code
        && content.starts_with(' ')
        && content.ends_with(' ')
        && !content.trim_matches(' ').is_empty();
    let content = if handlers.options().preformatted_code {
        handle_preformatted_code(&content)
    } else {
        content.trim_document_whitespace().to_string()
    };

    let delimiter = get_inline_code_delimiter(&content);
    let needs_padding =
        content.starts_with('`') || content.ends_with('`') || preserve_boundary_spaces;
    if needs_padding {
        Some(concat_strings!(delimiter, " ", content, " ", delimiter).into())
    } else {
        Some(concat_strings!(delimiter, content, delimiter).into())
    }
}

fn get_inline_code_delimiter(content: &str) -> String {
    let mut run_lengths = Vec::new();
    let mut current_run = 0;
    let mut longest_run = 0;

    for byte in content.bytes() {
        if byte == b'`' {
            current_run += 1;
            longest_run = longest_run.max(current_run);
        } else if current_run > 0 {
            run_lengths.push(current_run);
            current_run = 0;
        }
    }
    if current_run > 0 {
        run_lengths.push(current_run);
    }

    let delimiter_len = if content.starts_with('`') || content.ends_with('`') {
        longest_run + 1
    } else {
        (1..).find(|length| !run_lengths.contains(length)).unwrap()
    };

    "`".repeat(delimiter_len)
}

/// Newlines become spaces (+ an extra space if not in the middle of the code)
fn handle_preformatted_code(code: &str) -> String {
    let mut result = String::new();
    let mut is_prev_ch_new_line = false;
    let mut in_middle = false;
    for ch in code.chars() {
        if ch == '\n' {
            result.push(' ');
            is_prev_ch_new_line = true;
        } else {
            if is_prev_ch_new_line && !in_middle {
                result.push(' ');
            }
            result.push(ch);
            is_prev_ch_new_line = false;
            in_middle = true;
        }
    }
    if is_prev_ch_new_line {
        result.push(' ');
    }
    result
}
