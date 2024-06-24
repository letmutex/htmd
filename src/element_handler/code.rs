use crate::{
    node_util::get_parent_node_tag_name,
    options::{CodeBlockFence, CodeBlockStyle},
    text_util::{concat_strings, TrimAsciiWhitespace},
    Element,
};

pub(super) fn code_handler(element: Element) -> Option<String> {
    let is_code_block = get_parent_node_tag_name(&element.node).is_some_and(|t| &t == "pre");
    if is_code_block {
        handle_code_block(element)
    } else {
        handle_inline_code(element)
    }
}

fn handle_code_block(element: Element) -> Option<String> {
    let content = element.content;
    let content = content.strip_suffix("\n").unwrap_or(&content);
    if element.options.code_block_style == CodeBlockStyle::Fenced {
        let fence = if element.options.code_block_fence == CodeBlockFence::Tildes {
            get_code_fence_marker("~", content)
        } else {
            get_code_fence_marker("`", content)
        };
        let language = element
            .attrs
            .iter()
            .find(|attr| &attr.name.local == "class")
            .map(|attr| {
                attr.value
                    .to_string()
                    .split(" ")
                    .find(|cls| cls.starts_with("language-"))
                    .map(|lang| lang.split("-").skip(1).collect::<Vec<&str>>().join("-"))
            })
            .unwrap_or(None);
        let mut result = String::from(&fence);
        if language.is_some() {
            result.push_str(&language.unwrap());
        }
        result.push('\n');
        result.push_str(content);
        result.push('\n');
        result.push_str(&fence);
        Some(result)
    } else {
        let code = content
            .lines()
            .map(|line| concat_strings!("    ", line))
            .collect::<Vec<String>>()
            .join("\n");
        Some(code)
    }
}

fn get_code_fence_marker(symbol: &str, content: &str) -> String {
    let three_chars = symbol.repeat(3);
    if content.find(&three_chars).is_some() {
        let four_chars = symbol.repeat(4);
        if content.find(&four_chars).is_some() {
            symbol.repeat(5)
        } else {
            four_chars
        }
    } else {
        three_chars
    }
}

fn handle_inline_code(element: Element) -> Option<String> {
    // Case: <code>There is a literal backtick (`) here</code>
    //   to: ``There is a literal backtick (`) here``
    let mut use_double_backticks = false;
    // Case: <code>`starting with a backtick</code>
    //   to: `` `starting with a backtick ``
    let mut surround_with_spaces = false;
    let content = element.content;
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
    let content = if element.options.preformatted_code {
        handle_preformatted_code(&content)
    } else {
        content.trim_ascii_whitespace().to_string()
    };
    if use_double_backticks {
        if surround_with_spaces {
            Some(concat_strings!("`` ", content, " ``"))
        } else {
            Some(concat_strings!("``", content, "``"))
        }
    } else {
        Some(concat_strings!("`", content, "`"))
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
