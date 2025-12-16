use crate::{
    Element,
    dom_walker::is_block_element,
    element_handler::{HandlerResult, Handlers},
    node_util::parent_tag_name_equals,
    options::TranslationMode,
    text_util::concat_strings,
};
use html5ever::serialize::{HtmlSerializer, SerializeOpts, Serializer, TraversalScope, serialize};

use markup5ever_rcdom::{NodeData, SerializableHandle};
use std::io::{self, Write};

// A handler for tags whose only criteria (for faithful translation) is the tag
// name of the parent.
pub(super) fn handle_or_serialize_by_parent(
    handlers: &dyn Handlers,
    // The element to check.
    element: &Element,
    // A list of allowable tag names for this element's parent.
    tag_names: &Vec<&str>,
    // The value for `markdown_translate` to pass if this tag is markdown translatable.
    markdown_translated: bool,
) -> Option<HandlerResult> {
    // In faithful mode, fall back to HTML when this element's parent tag is not
    // in `tag_names` (e.g., `<tbody>` outside `<table>`, `<td>` outside `<tr>`, etc.).
    if handlers.options().translation_mode == TranslationMode::Faithful
        && !parent_tag_name_equals(element.node, tag_names)
    {
        Some(HandlerResult {
            content: serialize_element(handlers, element),
            markdown_translated: false,
        })
    } else {
        let content = handlers.walk_children(element.node).content;
        let content = content.trim_matches('\n');
        Some(HandlerResult {
            content: concat_strings!("\n\n", content, "\n\n"),
            markdown_translated,
        })
    }
}

// Given a node (which must be an element), serialize it (transform it back
// to HTML).
pub(crate) fn serialize_element(handlers: &dyn Handlers, element: &Element) -> String {
    let f = || -> io::Result<String> {
        let so = SerializeOpts {
            traversal_scope: TraversalScope::IncludeNode,
            ..Default::default()
        };
        let mut bytes = vec![];
        // If this is a block element, then serialize it and all its children.
        // Otherwise, serialize just this element, but use the current contents in
        // the place of children. This follows the Commonmark spec: [HTML
        // blocks](https://spec.commonmark.org/0.31.2/#html-blocks) contain only
        // HTML, not Markdown, while [raw HTML
        // inlines](https://spec.commonmark.org/0.31.2/#raw-html) contain Markdown.
        if !is_block_element(element.tag) {
            // Write this element's start tag.
            let NodeData::Element { name, attrs, .. } = &element.node.data else {
                return Err(io::Error::other("Not an element.".to_string()));
            };
            let mut ser = HtmlSerializer::new(&mut bytes, so.clone());
            ser.start_elem(
                name.clone(),
                attrs.borrow().iter().map(|at| (&at.name, &at.value[..])),
            )?;
            // Write out the contents, without escaping them. The standard serialization process escapes the contents, hence this manual approach.
            ser.writer
                .write_all(handlers.walk_children(element.node).content.as_bytes())?;
            // Write the end tag, if needed (HtmlSerializer logic will automatically omit this).
            ser.end_elem(name.clone())?;

            String::from_utf8(bytes).map_err(io::Error::other)
        } else {
            let sh: SerializableHandle = SerializableHandle::from(element.node.clone());
            serialize(&mut bytes, &sh, so)?;
            let s = String::from_utf8(bytes).map_err(io::Error::other)?;
            // We must avoid consecutive newlines in HTML blocks, since this
            // terminates the block per the CommonMark spec. Therefore, this
            // code replaces instances of two or more newlines with a single
            // newline, followed by escaped newlines. This is a hand-coded
            // version of the following regex:
            //
            // ```Rust
            // Regex::new(r#"(\r?\n\s*)(\r?\n\s*)"#).unwrap())
            //  .replace_all(&s, |caps: &Captures| {
            //      caps[1].to_string()
            //      + &(caps[2].replace("\r", "&#13;").replace("\n", "&#10;"))
            //  })
            // ```
            //
            // 1.  If the next character is an \\r or \\n, output it.
            // 2.  If the previous character was a \\r and the next
            //     character isn't a \\n, restart. Otherwise, output the
            //     \\n.
            // 3.  If the next character is whitespace but not \\n or \\r,
            //     output it then repeat this step.
            // 4.  If the next character is a \\r and the peeked following
            //     character isn't an \\n, output the \\r and restart.
            //     Otherwise, output an encoded \\r.
            // 5.  If the peeked next character is a \\n, output an encoded
            //     \\n. Otherwise, restart.
            // 6.  If the next character is whitespace but not \\n or \\r,
            //     output it then repeat this step. Otherwise, restart.
            //
            // Replace instances of two or more newlines with a newline
            // followed by escaped newlines
            let mut result = String::with_capacity(s.len());
            let mut chars = s.chars().peekable();

            while let Some(c) = chars.next() {
                // Step 1.
                if c == '\r' || c == '\n' {
                    result.push(c);

                    // Step 2.
                    if c == '\r' {
                        if chars.peek() == Some(&'\n') {
                            result.push(chars.next().unwrap());
                        } else {
                            continue;
                        }
                    }

                    // Step 3: Skip any whitespace after the newline.
                    while let Some(&next) = chars.peek() {
                        if next.is_whitespace() && next != '\r' && next != '\n' {
                            result.push(next);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    // Step 4.
                    if let Some(c) = chars.next() {
                        if c == '\r' || c == '\n' {
                            if c == '\r' {
                                if chars.peek() == Some(&'\n') {
                                    chars.next();
                                    result.push_str("&#13;&#10;");
                                } else {
                                    // Step 6.
                                    result.push('\r');
                                    continue;
                                }
                            } else {
                                result.push_str("&#10;");
                            }

                            // Step 6.
                            while let Some(&next) = chars.peek() {
                                if next.is_whitespace() && next != '\r' && next != '\n' {
                                    result.push(next);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        } else {
                            result.push(c);
                        }
                    }
                } else {
                    result.push(c);
                }
            }
            Ok(concat_strings!("\n\n", result, "\n\n"))
        }
    };
    match f() {
        Ok(s) => s,
        Err(err) => err.to_string(),
    }
}

// When in faithful translation mode, return an HTML translation if this element
// has more than the allowed number of attributes.
#[macro_export]
macro_rules! serialize_if_faithful {
    (
        // The handlers to use for serialization.
        $handlers: expr,
        // The element to translate.
        $element: expr,
        // The maximum number of attributes allowed for this element. Supply
        // -1 to serialize in faithful mode, even with no attributes.
        $num_attrs_allowed: expr
    ) => {
        if $handlers.options().translation_mode == $crate::options::TranslationMode::Faithful
            && $element.attrs.len() as i64 > $num_attrs_allowed
        {
            return Some($crate::element_handler::HandlerResult {
                content: $crate::element_handler::element_util::serialize_element(
                    $handlers, &$element,
                ),
                // This was translated using HTML, not Markdown.
                markdown_translated: false,
            });
        }
    };
}
