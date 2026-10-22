use html5ever::tendril::StrTendril;
use markup5ever_rcdom::{Node, NodeData};
use std::{iter::Peekable, str::Chars};

use crate::{
    Element,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
    util::{
        node::{copy_element_holding_text, get_parent_node},
        text::{concat_strings, trim_newlines},
    },
};

pub(super) fn span_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    // See the "Math" section of `translation_approach.md`.
    if element.attrs.len() == 1
        && let attr = &element.attrs[0]
        && *attr.name.local == *"class"
        && let Some(delimiter) = match &*attr.value {
            "math math-inline" => Some("$"),
            "math math-display" => Some("$$"),
            _ => None,
        }
        && let Some(text) = text_content(element.node)
    {
        let is_inline = delimiter == "$";
        let source = math_source(&text);
        let math = if is_inline {
            source.trim_matches(is_math_whitespace)
        } else {
            &source
        };
        if is_writable_math(math, is_inline) {
            return Some(concat_strings!(delimiter, math, delimiter).into());
        }
        // Serializing collapses the line endings of the original text, which
        // would run each of its comments on over the lines after it.
        let span = copy_element_holding_text(
            element.node,
            get_parent_node(element.node).as_ref(),
            StrTendril::from_slice(&source),
        );
        return translate_as_span(
            handlers,
            &Element {
                node: &span,
                ..*element
            },
        );
    }

    translate_as_span(handlers, element)
}

fn translate_as_span(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    // Always serialize as HTML if we're in faithful mode.
    serialize_if_extra_attrs!(handlers, element, -1);

    // Otherwise, just return the contents.
    let mut content = handlers.walk_children_content(element.node, element.context);
    if !element.context.literal {
        trim_newlines(&mut content);
    }

    Some(content.into())
}

/// The text of `node` when it is empty or holds a single text node; `None`
/// otherwise.
fn text_content(node: &Node) -> Option<String> {
    match node.children.borrow().as_slice() {
        [] => Some(String::new()),
        [child] => match &child.data {
            NodeData::Text { contents } => Some(contents.borrow().to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// pulldown-cmark's
/// [whitespace](https://github.com/pulldown-cmark/pulldown-cmark/blob/v0.13.4/pulldown-cmark/src/scanners.rs#L425-L427),
/// which, unlike the WHATWG's ASCII whitespace, includes a vertical tab: one
/// beside a `$`
/// [keeps it from opening or closing](https://github.com/pulldown-cmark/pulldown-cmark/blob/v0.13.4/pulldown-cmark/src/firstpass.rs#L951-L961)
/// inline math.
fn is_math_whitespace(char: char) -> bool {
    matches!(char, '\t' | '\n' | '\x0B' | '\x0C' | '\r' | ' ')
}

/// The math expression `text` spells, with its comments and line endings
/// replaced by spaces.
fn math_source(text: &str) -> String {
    let mut source = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(char) = chars.next() {
        match char {
            // A backslash escapes the character after it, so `\%` is a
            // literal percent sign while the `%` of `\\%` begins a comment. An
            // escaped line ending, LaTeX's control space, is left to the line
            // ending arm.
            '\\' => {
                source.push(char);
                if let Some(escaped) = chars.next_if(|&next| !is_line_ending(next)) {
                    source.push(escaped);
                }
            }
            // A comment runs through its line ending and the whitespace which
            // begins the next line.
            '%' => {
                skip_while(&mut chars, |next| !is_line_ending(next));
                if let Some(line_ending) = chars.next() {
                    skip_crlf_tail(line_ending, &mut chars);
                    skip_while(&mut chars, |next| matches!(next, ' ' | '\t'));
                }
                source.push(' ');
            }
            '\r' | '\n' => {
                skip_crlf_tail(char, &mut chars);
                source.push(' ');
            }
            _ => source.push(char),
        }
    }
    source
}

fn is_line_ending(char: char) -> bool {
    matches!(char, '\r' | '\n')
}

fn skip_while(chars: &mut Peekable<Chars>, is_skipped: impl Fn(char) -> bool) {
    while chars.next_if(|&next| is_skipped(next)).is_some() {}
}

/// Consumes the line feed of a CRLF pair, which is one line ending, after
/// `line_ending` has been read.
fn skip_crlf_tail(line_ending: char, chars: &mut Peekable<Chars>) {
    if line_ending == '\r' {
        chars.next_if_eq(&'\n');
    }
}

/// Whether pulldown-cmark reads `source` back between math delimiters as the
/// expression it is. An unescaped `$` would close the expression early, a
/// trailing unescaped `\` would escape the closing delimiter, and pulldown-cmark
/// pairs delimiters only around balanced braces.
fn is_writable_math(source: &str, is_inline: bool) -> bool {
    if is_inline && source.is_empty() {
        return false;
    }
    let mut brace_depth = 0_usize;
    let mut chars = source.chars();
    while let Some(char) = chars.next() {
        match char {
            '\\' => {
                if chars.next().is_none() {
                    return false;
                }
            }
            '$' => return false,
            '{' => brace_depth += 1,
            '}' => match brace_depth.checked_sub(1) {
                Some(depth) => brace_depth = depth,
                None => return false,
            },
            _ => {}
        }
    }
    brace_depth == 0
}
