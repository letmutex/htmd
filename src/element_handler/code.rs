use std::rc::Rc;

use html5ever::Attribute;
use markup5ever_rcdom::{Node, NodeData};

use crate::{
    Element,
    element_handler::{HandlerResult, Handlers, serialize_element},
    node_util::{get_node_tag_name, get_parent_node},
    options::{CodeBlockFence, CodeBlockStyle, TranslationMode},
    serialize_if_faithful,
    text_util::{JoinOnStringIterator, TrimAsciiWhitespace, concat_strings},
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
    let three_chars = symbol.repeat(3);
    if content.contains(&three_chars) {
        let four_chars = symbol.repeat(4);
        if content.contains(&four_chars) {
            symbol.repeat(5)
        } else {
            four_chars
        }
    } else {
        three_chars
    }
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
    // Case: <code>There is a literal backtick (`) here</code>
    //   to: ``There is a literal backtick (`) here``
    let mut use_double_backticks = false;
    // Case: <code>`starting with a backtick</code>
    //   to: `` `starting with a backtick ``
    let mut surround_with_spaces = false;
    let content = handlers.walk_children(element.node).content;
    let chars = content.chars().collect::<Vec<char>>();
    let len = chars.len();
    for (idx, c) in chars.iter().enumerate() {
        if c == &'`' {
            let prev = if idx > 0 { chars[idx - 1] } else { '\0' };
            let next = if idx < len - 1 { chars[idx + 1] } else { '\0' };
            if prev != '`' && next != '`' {
                use_double_backticks = true;
                surround_with_spaces = idx == 0;
                break;
            }
        }
    }
    let content = if handlers.options().preformatted_code {
        handle_preformatted_code(&content)
    } else {
        content.trim_ascii_whitespace().to_string()
    };
    if use_double_backticks {
        if surround_with_spaces {
            Some(concat_strings!("`` ", content, " ``").into())
        } else {
            Some(concat_strings!("``", content, "``").into())
        }
    } else {
        Some(concat_strings!("`", content, "`").into())
    }
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
