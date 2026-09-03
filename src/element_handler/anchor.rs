use std::cell::RefCell;

use crate::{
    Element, ElementHandler,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
    options::{LinkReferenceStyle, LinkStyle},
    text_util::{StripWhitespace, concat_strings, normalize_title},
};

/// Handler for HTML `<a>` (anchor) elements.
///
/// Converts anchor tags to Markdown links (inlined, autolinks, or reference-style links).
///
/// # State & Limitations
///
/// When using [`LinkStyle::Referenced`], link reference definitions (e.g. `[1]: https://...`)
/// are collected in a thread-local buffer during DOM traversal and drained when [`append`](ElementHandler::append)
/// is called at the end of the document conversion.
///
/// **Limitations:**
/// - **Thread-local buffering:** State is isolated per-thread, making sharing [`HtmlToMarkdown`](crate::HtmlToMarkdown)
///   across threads safe. However, nested or re-entrant conversions on the *same* thread (such as invoking
///   `convert` inside a custom element handler) will share this thread-local buffer if both use
///   reference-style links.
/// - **Speculative conversion:** If a container handler (such as a table) converts children speculatively
///   and then discards the result in faithful mode, any reference-style links inside those children
///   will remain in the buffer for document-level append unless handled.
pub(super) struct AnchorElementHandler {}

impl AnchorElementHandler {
    thread_local! {
        static LINK_REFERENCES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn new() -> Self {
        Self {}
    }
}

impl ElementHandler for AnchorElementHandler {
    fn append(&self) -> Option<String> {
        AnchorElementHandler::LINK_REFERENCES.with(|links| {
            let mut links = links.borrow_mut();
            if links.is_empty() {
                return None;
            }

            let links = std::mem::take(&mut *links);
            let content_len: usize = links.iter().map(String::len).sum();
            let mut result = String::with_capacity(content_len + links.len().saturating_add(1));
            result.push_str("\n\n");
            for (index, link) in links.iter().enumerate() {
                if index > 0 {
                    result.push('\n');
                }
                result.push_str(link);
            }
            result.push_str("\n\n");
            Some(result)
        })
    }

    fn handle(&self, handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
        let mut link: Option<String> = None;
        let mut title: Option<String> = None;
        for attr in element.attrs.iter() {
            let name = &attr.name.local;
            if name == "href" {
                link = Some(attr.value.to_string())
            } else if name == "title" {
                title = Some(attr.value.to_string());
            } else {
                // This is an attribute which can't be translated to Markdown.
                serialize_if_extra_attrs!(handlers, element, 0);
            }
        }

        let Some(link) = link else {
            return Some(handlers.walk_children(element.node));
        };

        // Handle new lines in title
        let title = title.as_deref().map(normalize_title);

        let link = escape_link_destination(link);

        let content = handlers.walk_children(element.node).content;
        let md = match handlers.options().link_style {
            LinkStyle::Inlined => {
                self.build_inlined_anchor(&content, &link, title.as_deref(), false)
            }
            LinkStyle::InlinedPreferAutolinks => {
                self.build_inlined_anchor(&content, &link, title.as_deref(), true)
            }
            LinkStyle::Referenced => self.build_referenced_anchor(
                &content,
                link,
                title,
                &handlers.options().link_reference_style,
            ),
        };

        Some(md.into())
    }
}

impl AnchorElementHandler {
    fn build_inlined_anchor(
        &self,
        content: &str,
        link: &str,
        title: Option<&str>,
        prefer_autolinks: bool,
    ) -> String {
        if prefer_autolinks && content == link {
            let mut result = String::with_capacity(link.len() + 2);
            result.push('<');
            result.push_str(link);
            result.push('>');
            return result;
        }

        let has_spaces_in_link = link.contains(' ');
        let (content, _) = content.strip_leading_document_whitespace();
        let (content, trailing_whitespace) = content.strip_trailing_document_whitespace();
        let title_len = title.map_or(0, |t| t.len() + 3);
        let trailing_len = trailing_whitespace.map_or(0, str::len);
        let wrapper_len = if has_spaces_in_link { 2 } else { 0 };
        let mut result = String::with_capacity(
            content.len() + link.len() + title_len + trailing_len + wrapper_len + 4,
        );
        result.push('[');
        result.push_str(content);
        result.push_str("](");
        if has_spaces_in_link {
            result.push('<');
        }
        result.push_str(link);
        if has_spaces_in_link {
            result.push('>');
        }
        if let Some(title) = title {
            result.push_str(" \"");
            result.push_str(title);
            result.push('"');
        }
        result.push(')');
        if let Some(trailing_whitespace) = trailing_whitespace {
            result.push_str(trailing_whitespace);
        }
        result
    }

    fn build_referenced_anchor(
        &self,
        content: &str,
        link: String,
        title: Option<String>,
        style: &LinkReferenceStyle,
    ) -> String {
        AnchorElementHandler::LINK_REFERENCES.with(|links| {
            let mut links = links.borrow_mut();
            let index = links.len() + 1;
            let title = title
                .as_deref()
                .map_or(String::new(), |t| format!(" \"{t}\""));
            let (current, append) = match style {
                LinkReferenceStyle::Full => (
                    concat_strings!("[", content, "][", index.to_string(), "]"),
                    concat_strings!("[", index.to_string(), "]: ", link, title),
                ),
                LinkReferenceStyle::Collapsed => (
                    concat_strings!("[", content, "][]"),
                    concat_strings!("[", content, "]: ", link, title),
                ),
                LinkReferenceStyle::Shortcut => (
                    concat_strings!("[", content, "]"),
                    concat_strings!("[", content, "]: ", link, title),
                ),
            };
            links.push(append);
            current
        })
    }
}

fn escape_link_destination(link: String) -> String {
    if !link.contains(['(', ')']) {
        return link;
    }

    let mut escaped = String::with_capacity(link.len());
    for ch in link.chars() {
        match ch {
            '(' => escaped.push_str("\\("),
            ')' => escaped.push_str("\\)"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
