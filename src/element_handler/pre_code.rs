use html5ever::Attribute;

use crate::{
    element_handler::{
        Context, Element, HandlerResult, Handlers,
        element_util::{
            serialize_element, serialize_element_result, serialize_element_when_faithful,
            serialize_if_extra_attrs, serialize_if_extra_attrs_or_inline, serialize_when_faithful,
        },
    },
    node::{Node, NodeType},
    options::{CodeBlockFence, CodeBlockStyle, Options, TranslationMode},
    util::{
        escape::has_line_ending,
        node::{get_node_tag_name, is_inside_pre, parent_tag_name_equals},
        text::{JoinOnStringIterator, TrimDocumentWhitespace, concat_strings, frame_as_block},
    },
};

pub(super) fn pre_code_handler(
    handlers: &dyn Handlers,
    element: &Element,
) -> Option<HandlerResult> {
    if element.tag == "pre" {
        handle_pre(handlers, element)
    } else {
        handle_code(handlers, element)
    }
}

fn handle_pre(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    // A code block is a CommonMark block, so it needs a block context. In an
    // inline context this `<pre>` is a raw HTML inline instead, its content
    // walked like that of any other one.
    serialize_if_extra_attrs_or_inline!(handlers, element, 0);

    let options = handlers.options();
    let is_pure = options.translation_mode == TranslationMode::Pure;

    if is_pure
        && options.bare_pre_as_code_block
        && element.context.is_block()
        && !element.context.literal
        && !is_inside_pre(element.node)
        && !has_code_block_descendant(element.node)
    {
        let child_context = Context {
            literal: true,
            ..element.context
        };
        // Walk direct children here rather than via `walk_children` so direct
        // text nodes skip `dom_walker`'s bare-`<pre>` fence escaping while
        // unwrapped `<pre>` paths keep it intact.
        let mut content = String::new();
        for child in element.node.children().iter() {
            if let Some(text) = child.text() {
                content.push_str(&text);
            } else if let Some(result) = handlers.handle(&child, child_context) {
                content.push_str(&result.content);
            }
        }
        let language = (options.code_block_style == CodeBlockStyle::Fenced)
            .then(|| find_language_from_attrs(element.attrs))
            .flatten();
        let code_block = format_code_block(&content, language, options);
        return Some(frame_as_block(&code_block).into());
    }

    // The only faithful translation for this is from
    // `<pre><code>blah</code></pre>` to a code block. So, check that this node
    // has only one element, a pure `<code>` element. Cases:
    //
    // 1.  We're in pure translation mode. No special treatment.
    // 2.  We're in faithful mode:
    //     1.  The child is pure, consists of one element which is a code tag.
    //         No special treatment.
    //     2.  All other cases: produce HTML.
    let is_simple_code_block = {
        let children_view = element.node.children();
        let mut children = children_view.iter();
        children.next().is_some_and(|first| {
            children.next().is_none() && get_node_tag_name(&first) == Some("code")
        })
    };

    if is_pure || is_simple_code_block {
        let result = handlers.walk_children(
            element.node,
            Context {
                literal: true,
                ..element.context
            },
        );

        serialize_when_faithful!(
            handlers,
            !result.markdown_translated,
            serialize_element(handlers, element)
        );

        let skip_framing = is_pure
            && options.bare_pre_as_code_block
            && element.context.literal
            && parent_tag_name_equals(element.node, &["pre"])
            && element.node.prev_sibling().is_none()
            && element.node.next_sibling().is_none();
        let content = if skip_framing {
            result.content
        } else {
            frame_as_block(&result.content)
        };
        Some(content.into())
    } else {
        Some(serialize_element_result(handlers, element))
    }
}

fn has_code_block_descendant(node: &Node<'_>) -> bool {
    let is_pre = get_node_tag_name(node) == Some("pre");
    node.children().iter().any(|child| {
        (is_pre && get_node_tag_name(&child) == Some("code")) || has_code_block_descendant(&child)
    })
}

fn handle_code(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    // In faithful mode, all children of a code tag must be text to translate
    // as markdown.
    if handlers.options().translation_mode == TranslationMode::Faithful
        && !element
            .node
            .children()
            .iter()
            .all(|node| node.node_type() == NodeType::Text)
    {
        return Some(serialize_element_result(handlers, element));
    }

    // Determine the type: inline code or a code block.
    let parent_node = element.node.parent();
    let is_code_block = parent_node
        .as_ref()
        .is_some_and(|parent| get_node_tag_name(parent) == Some("pre"));
    if is_code_block {
        handle_code_block(handlers, element, &parent_node.unwrap())
    } else {
        handle_inline_code(handlers, element)
    }
}

fn handle_code_block(
    handlers: &dyn Handlers,
    element: &Element,
    parent: &Node<'_>,
) -> Option<HandlerResult> {
    // A code block is a CommonMark block, so it needs a block context.
    serialize_element_when_faithful!(handlers, element, element.context.is_inline());
    let content = handlers.walk_children_content(
        element.node,
        Context {
            literal: true,
            ..element.context
        },
    );
    let options = handlers.options();
    let parent_class;
    let language = if options.code_block_style == CodeBlockStyle::Fenced {
        let language = match find_language_from_attrs(element.attrs) {
            Some(lang) => Some(lang),
            None => {
                parent_class = parent.attr("class");
                parent_class.as_deref().and_then(find_language_from_class)
            }
        };
        serialize_if_extra_attrs!(handlers, element, if language.is_none() { 0 } else { 1 });
        language
    } else {
        serialize_if_extra_attrs!(handlers, element, 0);
        None
    };
    Some(format_code_block(&content, language, options).into())
}

fn format_code_block(content: &str, language: Option<&str>, options: &Options) -> String {
    let content = content.strip_suffix('\n').unwrap_or(content);
    if options.code_block_style == CodeBlockStyle::Fenced {
        let fence = if options.code_block_fence == CodeBlockFence::Tildes {
            get_code_fence_marker("~", content)
        } else {
            get_code_fence_marker("`", content)
        };
        let mut result = String::from(&fence);
        if let Some(lang) = language {
            result.push_str(lang);
        }
        result.push('\n');
        result.push_str(content);
        result.push('\n');
        result.push_str(&fence);
        result
    } else {
        content
            .lines()
            .map(|line| concat_strings!("    ", line))
            .join("\n")
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

fn find_language_from_attrs(attrs: &[Attribute]) -> Option<&str> {
    attrs
        .iter()
        .find(|attr| &attr.name.local == "class")
        .and_then(|attr| find_language_from_class(&attr.value))
}

fn find_language_from_class(class_attr: &str) -> Option<&str> {
    class_attr.split_ascii_whitespace().find_map(|class| {
        class
            .strip_prefix("language-")
            .filter(|language| !language.is_empty())
    })
}

fn handle_inline_code(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(handlers, element, 0);
    // A code span holds literal text — every child is a text node here, as
    // `handle_code` checked — so the context it is read in doesn't matter.
    let content = handlers.walk_children_content(
        element.node,
        Context {
            literal: true,
            ..element.context
        },
    );
    let preserve_boundary_spaces = handlers.options().preformatted_code
        && content.starts_with(' ')
        && content.ends_with(' ')
        && !content.trim_matches(' ').is_empty();
    let content = if handlers.options().preformatted_code {
        handle_preformatted_code(&content)
    } else {
        content.trim_document_whitespace().to_string()
    };
    // A code span's content is literal text, so a line ending written in one
    // stays a line ending and ends the leaf block the span sits in; no encoding
    // of it survives there. Neither has CommonMark an empty code span: the
    // backticks meant to open one close it instead, leaving them literal text
    // and the element gone. Either way it goes out as a raw HTML inline, whose
    // contents the walk collapses. See the "Code blocks and code spans" section
    // of `translation_approach.md`. The trim above can empty the content, so
    // this runs after it.
    serialize_element_when_faithful!(
        handlers,
        element,
        has_line_ending(&content) || content.is_empty()
    );
    // Pure mode has no fallback, and drops the span rather than write the bare
    // backticks CommonMark reads as literal text.
    if content.is_empty() {
        return None;
    }
    // Only pure mode reaches a line ending here, the fallback above having
    // taken faithful mode's. CommonMark reads a line ending in a code span as a
    // space, so writing that space is what the span meant anyway, and it keeps
    // the bare newline out of the heading it would truncate.
    let content = if has_line_ending(&content) {
        content.replace(['\r', '\n'], " ")
    } else {
        content
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
    let mut observed_run_lengths = vec![false; 2];
    let mut current_run = 0;
    let mut longest_run = 0;

    for byte in content.bytes() {
        if byte == b'`' {
            current_run += 1;
            longest_run = longest_run.max(current_run);
        } else if current_run > 0 {
            observed_run_lengths.resize(observed_run_lengths.len().max(current_run + 2), false);
            observed_run_lengths[current_run] = true;
            current_run = 0;
        }
    }
    if current_run > 0 {
        observed_run_lengths.resize(observed_run_lengths.len().max(current_run + 2), false);
        observed_run_lengths[current_run] = true;
    }

    let delimiter_len = if content.starts_with('`') || content.ends_with('`') {
        longest_run + 1
    } else {
        observed_run_lengths
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(length, observed)| (!observed).then_some(length))
            .unwrap_or(longest_run + 1)
    };

    "`".repeat(delimiter_len)
}

/// Newlines become spaces (+ an extra space if not in the middle of the code)
fn handle_preformatted_code(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
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
