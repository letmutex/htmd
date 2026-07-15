use std::cell::RefCell;

use crate::{
    Element, ElementHandler,
    element_handler::{HandlerResult, Handlers},
    options::{LinkReferenceStyle, LinkStyle},
    serialize_if_faithful,
    text_util::{StripWhitespace, TrimDocumentWhitespace, concat_strings},
};

pub(super) struct AnchorElementHandler {}

impl AnchorElementHandler {
    thread_local! {
        static LINKS: RefCell<Vec<String>> = const { RefCell::new(vec![]) };
    }
}

impl ElementHandler for AnchorElementHandler {
    fn append(&self) -> Option<String> {
        AnchorElementHandler::LINKS.with(|links| {
            let mut links = links.borrow_mut();
            if links.is_empty() {
                return None;
            }
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
            links.clear();
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
                serialize_if_faithful!(handlers, element, 0);
            }
        }

        let Some(link) = link else {
            return Some(handlers.walk_children(element.node));
        };

        // Handle new lines in title
        let title = title.map(|text| process_title(&text));

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
    pub(super) fn new() -> Self {
        Self {}
    }

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
        AnchorElementHandler::LINKS.with(|links| {
            let title = title
                .as_deref()
                .map_or(String::new(), |t| format!(" \"{t}\""));
            let (current, append) = match style {
                LinkReferenceStyle::Full => {
                    let index = links.borrow().len() + 1;
                    (
                        concat_strings!("[", content, "][", index.to_string(), "]"),
                        concat_strings!("[", index.to_string(), "]: ", link, title),
                    )
                }
                LinkReferenceStyle::Collapsed => (
                    concat_strings!("[", content, "][]"),
                    concat_strings!("[", content, "]: ", link, title),
                ),
                LinkReferenceStyle::Shortcut => (
                    concat_strings!("[", content, "]"),
                    concat_strings!("[", content, "]: ", link, title),
                ),
            };
            links.borrow_mut().push(append);
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

fn process_title(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut wrote_any = false;

    for line in text.lines() {
        let line = line.trim_document_whitespace();
        if line.is_empty() {
            continue;
        }
        if wrote_any {
            result.push('\n');
        }
        for ch in line.chars() {
            if ch == '"' {
                result.push('\\');
            }
            result.push(ch);
        }
        wrote_any = true;
    }

    result
}
