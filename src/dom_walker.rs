use std::borrow::Cow;

use crate::{
    Context, Element, Node, NodeType,
    dom::{ChildCursor, DomNode, SyntheticTextElement},
    element_handler::{
        ElementHandlers,
        element_util::{math_delimiter, serialize_element},
    },
    node::NodeInner,
};

use super::{
    options::TranslationMode,
    util::{
        escape::{
            escape_html, escape_markdown_inline_special, index_of_markdown_ordered_item_delimiter,
            is_markdown_atx_heading, push_escaped_ascii,
        },
        node::is_block_element,
        text::{
            TrimDocumentWhitespace, compress_whitespace, concat_strings, frame_as_block,
            trim_newlines,
        },
    },
};

pub(crate) fn walk_node(
    node: &Node<'_>,
    output: &mut String,
    handlers: &ElementHandlers,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    context: Context,
) -> bool {
    match &node.0 {
        #[cfg(feature = "ego-tree")]
        NodeInner::Ego(n) => walk_node_generic(
            *n,
            output,
            handlers,
            parent_tag,
            trim_leading_spaces,
            context,
        ),
        #[cfg(feature = "rcdom")]
        NodeInner::Rc(n) => walk_node_generic(
            n.as_rc(),
            output,
            handlers,
            parent_tag,
            trim_leading_spaces,
            context,
        ),
        NodeInner::SyntheticElement(synth) => {
            walk_element(node, synth.tag, synth.attrs, output, handlers, context)
        }
        NodeInner::SyntheticText(synth) => {
            walk_text(synth.text, output, parent_tag, trim_leading_spaces, context)
        }
    }
}

fn walk_node_generic<N: DomNode>(
    node: N,
    output: &mut String,
    handlers: &ElementHandlers,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    context: Context,
) -> bool {
    match node.node_type() {
        NodeType::Document => {
            let _ = walk_children_generic(node, output, handlers, true, Context::BLOCK);
            trim_output_end(output);
            true
        }
        NodeType::Text => {
            node.with_text(|text| walk_text(text, output, parent_tag, trim_leading_spaces, context))
        }
        NodeType::Element => node.with_element(|tag, _qual_name, attrs| {
            let wrapped = node.as_node();
            walk_element(&wrapped, tag, attrs, output, handlers, context)
        }),
        NodeType::Comment => {
            if handlers.options.translation_mode == TranslationMode::Faithful {
                node.with_comment(|contents| walk_comment(contents, output, context));
            }
            true
        }
        NodeType::Doctype | NodeType::ProcessingInstruction => true,
    }
}

/// A comment is an
/// [HTML block](https://spec.commonmark.org/0.31.2/#html-blocks) of type 2,
/// which ends at its own `-->` rather than at a blank line: in a block context
/// it is framed as a block and its line endings are left as they are.
///
/// In an inline context it is a raw HTML inline, whose whitespace the
/// "Translating HTML nodes" section of `translation_approach.md` collapses.
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
    node: &Node<'_>,
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
        if !context.literal {
            trim_newlines(&mut content);
        }
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
    tag == "span" && handlers.can_passthrough_span && math_delimiter(attrs).is_none()
}

pub(crate) fn walk_children(
    node: &Node<'_>,
    output: &mut String,
    handlers: &ElementHandlers,
    is_parent_block_element: bool,
    // The context the children are walked in.
    context: Context,
    // Return value: `markdown_translated`.
) -> bool {
    match &node.0 {
        #[cfg(feature = "ego-tree")]
        NodeInner::Ego(n) => {
            walk_children_generic(*n, output, handlers, is_parent_block_element, context)
        }
        #[cfg(feature = "rcdom")]
        NodeInner::Rc(n) => walk_children_generic(
            n.as_rc(),
            output,
            handlers,
            is_parent_block_element,
            context,
        ),
        NodeInner::SyntheticElement(synth) => {
            let trim_leading_spaces = !context.literal && is_parent_block_element;
            walk_text(
                synth.text,
                output,
                Some(synth.tag),
                trim_leading_spaces,
                context,
            )
        }
        NodeInner::SyntheticText(_) => true,
    }
}

fn walk_children_generic<N: DomNode>(
    node: N,
    output: &mut String,
    handlers: &ElementHandlers,
    is_parent_block_element: bool,
    context: Context,
) -> bool {
    let mut trim_leading_spaces = !context.literal && is_parent_block_element;
    let tag = match node.node_type() {
        NodeType::Document => Some("html"),
        NodeType::Element => node.tag_name(),
        _ => None,
    };
    let mut markdown_translated = true;

    node.with_children_cursor(|mut cursor| {
        while let Some(curr) = cursor.next() {
            if cursor
                .peek()
                .is_some_and(|next| curr.can_combine_with(next))
            {
                if handlers.options.translation_mode == TranslationMode::Pure {
                    let mut combined_text = String::new();
                    curr.push_single_text(&mut combined_text);
                    let mut prev = curr;
                    while let Some(next) = cursor.peek() {
                        if !prev.can_combine_with(next) {
                            break;
                        }
                        let next = cursor.next().unwrap();
                        next.push_single_text(&mut combined_text);
                        prev = next;
                    }

                    curr.with_element(|el_tag, qual_name, attrs| {
                        let synth = SyntheticTextElement {
                            parent: Some(node.as_node()),
                            tag: el_tag,
                            qual_name,
                            attrs,
                            text: &combined_text,
                        };
                        let synth_node = Node::from_synthetic_element(&synth);
                        let is_block = is_block_element(el_tag);
                        let trims_spaces = is_block && !context.literal;
                        if trims_spaces {
                            trim_output_end_spaces(output);
                        }
                        let output_len = output.len();
                        markdown_translated &=
                            walk_element(&synth_node, el_tag, attrs, output, handlers, context);
                        if output.len() > output_len {
                            trim_leading_spaces = trims_spaces;
                        }
                    });
                    continue;
                }

                // Combining writes a run of elements as one, which faithful mode
                // owes the reader as the elements they were. Writing them one after
                // the other is no answer either, since that is what combining
                // avoids: `*a**b*` is a single emphasis holding `a**b`, and
                // `` `a``b` `` a single code span. Each element goes out as HTML.
                append_serialized_inline(curr, output, handlers, context);
                let mut prev = curr;
                while let Some(next) = cursor.peek() {
                    if !prev.can_combine_with(next) {
                        break;
                    }
                    let next = cursor.next().unwrap();
                    append_serialized_inline(next, output, handlers, context);
                    prev = next;
                }
                markdown_translated = false;
                trim_leading_spaces = false;
                continue;
            }

            let is_block = curr.tag_name().is_some_and(is_block_element);

            // Preformatted content keeps every space it holds, so a block inside
            // one trims nothing.
            let trims_spaces = is_block && !context.literal;

            if trims_spaces {
                // Trim trailing spaces for the previous element
                trim_output_end_spaces(output);
            }

            let output_len = output.len();

            markdown_translated &=
                walk_node_generic(curr, output, handlers, tag, trim_leading_spaces, context);

            if output.len() > output_len {
                // Something was appended, update the flag
                trim_leading_spaces = trims_spaces;
            }
        }
    });

    markdown_translated
}

/// Appends `node`, an element of a combinable run, to `output` as a raw HTML
/// inline. Only inline elements form a run, so the serializer's block spelling
/// never applies here.
fn append_serialized_inline<N: DomNode>(
    node: N,
    output: &mut String,
    handlers: &ElementHandlers,
    context: Context,
) {
    node.with_element(|tag, _qual_name, attrs| {
        let wrapped = node.as_node();
        let element = Element {
            node: &wrapped,
            tag,
            attrs,
            context,
            skipped_handlers: 0,
        };
        let html = serialize_element(handlers, &element);
        append_normalized_content(output, html, context.literal);
    });
}

/// Normalizes content before adding to output by:
/// 1. Collapsing excessive newlines (max 2 consecutive newlines)
/// 2. Collapsing adjacent spaces between inline elements
///
/// When `preserve_whitespace` is true, the content is appended without normalization.
fn append_normalized_content(output: &mut String, content: String, preserve_whitespace: bool) {
    if output.is_empty() {
        *output = content;
        return;
    }

    if preserve_whitespace {
        output.push_str(&content);
        return;
    }

    // Same newline-count idiom as `join_blocks`: len after trim_matches.
    let last_newlines = output.len() - output.trim_end_matches('\n').len();
    let content_newlines = content.len() - content.trim_start_matches('\n').len();
    let total_newlines = last_newlines + content_newlines;

    let content_slice = if total_newlines > 2 {
        let to_remove = std::cmp::min(total_newlines - 2, content_newlines);
        &content[to_remove..]
    } else {
        &content[..]
    };

    // Collapse adjacent spaces between inline elements
    let content_slice = if last_newlines == 0
        && content_newlines == 0
        && output.ends_with(' ')
        && content_slice.starts_with(' ')
    {
        &content_slice[1..]
    } else {
        content_slice
    };

    escape_image_marker(output, content_slice);
    output.push_str(content_slice);
}

/// Escapes a `!` ending `output` when `content` begins with a `[`, which would
/// otherwise read as the start of an image rather than a link.
fn escape_image_marker(output: &mut String, content: &str) {
    if !content.starts_with('[') {
        return;
    }
    let Some(before_marker) = output.strip_suffix('!') else {
        return;
    };
    let preceding_backslashes = before_marker.len() - before_marker.trim_end_matches('\\').len();
    if preceding_backslashes % 2 == 0 {
        output.insert(before_marker.len(), '\\');
    }
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
/// '&copy;'   -> '\&copy;'   // entity reference
/// '$5'       -> '\$5'       // math delimiter
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
    let Some(first) = text.as_bytes().first().copied() else {
        return text;
    };

    let need_escape = matches!(first, b'=' | b'~' | b'>' | b'-' | b'+' | b'#' | b'0'..=b'9')
        || text
            .bytes()
            .any(|b| escape_markdown_inline_special(b).is_some());

    if !need_escape {
        return escape_html(text);
    }

    // Decide structural leading escapes on the raw input before rewriting
    // specials: the specials pass does not alter ATX `#` prefixes or the
    // second-char space of list markers, so pre-escape checks match the old
    // post-escape behavior without `insert(0, ...)`.
    let needs_leading_backslash = match first {
        b'=' | b'~' | b'>' => true,
        // A line of nothing but `-` and spaces is also a thematic break or a
        // setext underline. Anything else after the second `-` -- `--foo` --
        // is neither, and opens no list either.
        b'-' => {
            matches!(text.as_bytes().get(1), Some(b' '))
                || text.bytes().all(|byte| matches!(byte, b'-' | b' '))
        }
        b'+' => matches!(text.as_bytes().get(1), None | Some(b' ')),
        b'#' => is_markdown_atx_heading(text.as_ref()),
        _ => false,
    };

    let mut escaped = String::with_capacity(text.len() + 8 + usize::from(needs_leading_backslash));
    if needs_leading_backslash {
        escaped.push('\\');
    }
    push_escaped_ascii(text.as_ref(), &mut escaped, escape_markdown_inline_special);

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
