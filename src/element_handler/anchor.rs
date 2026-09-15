use std::cell::RefCell;

use crate::{
    Element, ElementHandler, Options,
    element_handler::EventSubscription,
    element_handler::EventTypes,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
    options::{LinkReferenceStyle, LinkStyle},
    util::{
        escape::{escape_link_destination, normalize_title},
        text::{StripWhitespace, concat_strings},
    },
};

/// Handler for HTML `<a>` (anchor) elements.
///
/// Converts anchor tags to Markdown links (inlined, autolinks, or reference-style links).
///
/// When using [`LinkStyle::Referenced`], link reference definitions (e.g. `[1]: https://...`)
/// are collected in a scoped thread-local buffer during DOM traversal.
/// Nested conversions enter a new scope frame, and speculative container conversions
/// (like tables or pre blocks falling back to raw HTML) roll back discarded links.
pub(super) struct AnchorElementHandler {}

#[derive(Default)]
struct AnchorScope {
    references: Vec<String>,
    checkpoints: Vec<usize>,
}

impl AnchorElementHandler {
    thread_local! {
        static SCOPES: RefCell<Vec<AnchorScope>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn new() -> Self {
        Self {}
    }

    fn with_current_scope_mut<R>(f: impl FnOnce(&mut AnchorScope) -> R) -> R {
        AnchorElementHandler::SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            if scopes.is_empty() {
                scopes.push(AnchorScope::default());
            }
            let last = scopes.last_mut().unwrap();
            f(last)
        })
    }
}

impl ElementHandler for AnchorElementHandler {
    fn event_subscription(&self, options: &Options) -> EventSubscription {
        if options.link_style == LinkStyle::Referenced {
            EventSubscription {
                events: EventTypes::DOC_EVENTS | EventTypes::ELEMENT_EVENTS,
                tags: &["table", "pre", "ol", "ul"],
            }
        } else {
            EventSubscription::NONE
        }
    }

    fn on_doc_enter(&self) {
        AnchorElementHandler::SCOPES.with(|scopes| {
            scopes.borrow_mut().push(AnchorScope::default());
        });
    }

    fn on_doc_leave(&self) {
        AnchorElementHandler::SCOPES.with(|scopes| {
            scopes.borrow_mut().pop();
        });
    }

    fn on_element_enter(&self, _element: &Element) {
        AnchorElementHandler::SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            if let Some(scope) = scopes.last_mut() {
                scope.checkpoints.push(scope.references.len());
            }
        });
    }

    fn on_element_leave(&self, _element: &Element, result: Option<&HandlerResult>) {
        AnchorElementHandler::SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            if let Some(scope) = scopes.last_mut()
                && let Some(checkpoint) = scope.checkpoints.pop()
            {
                let is_markdown_translated = result.is_some_and(|r| r.markdown_translated);
                if !is_markdown_translated {
                    scope.references.truncate(checkpoint);
                }
            }
        });
    }

    fn append(&self) -> Option<String> {
        AnchorElementHandler::SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            let scope = scopes.last_mut()?;
            if scope.references.is_empty() {
                return None;
            }

            let links = std::mem::take(&mut scope.references);
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

    fn handle(&self, handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
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
            return Some(handlers.walk_children(element.node, element.context));
        };

        // Handle new lines in title
        let title = title.as_deref().map(normalize_title);

        let link = escape_link_destination(link);

        let content = handlers.walk_children_content(element.node, element.context);
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
        AnchorElementHandler::with_current_scope_mut(|scope| {
            let index = scope.references.len() + 1;
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
            scope.references.push(append);
            current
        })
    }
}
