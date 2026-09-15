mod anchor;
mod blockquote;
mod br;
mod caption;
mod code;
pub(crate) mod element_util;
mod emphasis;
mod head_body;
mod headings;
mod hr;
mod html;
mod img;
mod li;
mod list;
mod p;
mod pre;
mod span;
mod table;
mod table_section;
mod td_th;
mod tr;

use crate::{
    dom_walker::walk_node,
    element_handler::element_util::serialize_element_result,
    options::{Options, TranslationMode},
    util::text::frame_as_block,
};

use super::{Context, Element};
use anchor::AnchorElementHandler;
use blockquote::blockquote_handler;
use br::br_handler;
use caption::caption_handler;
use code::code_handler;
use emphasis::emphasis_handler;
use head_body::head_body_handler;
use headings::headings_handler;
use hr::hr_handler;
use html::html_handler;
use html5ever::Attribute;
use img::img_handler;
use li::list_item_handler;
use list::list_handler;
use markup5ever_rcdom::Node;
use p::p_handler;
use pre::pre_handler;
use span::span_handler;
use std::{collections::HashMap, rc::Rc};
use table::table_handler;
use table_section::table_section_handler;
use td_th::td_th_handler;
use tr::tr_handler;

/// The processing result of an `ElementHandler`.
pub struct HandlerResult {
    /// The converted content.
    pub content: String,
    /// When true, this element was translated using Markdown,
    /// not HTML. This is only needed in faithful translation mode (see the
    /// `Options`): for code blocks, translating a `<pre><code>` sequence to
    /// Markdown, not HTML, requires a Markdown translated `<code>` block;
    /// likewise, translating lists ((`<ol>`/`<ul>`)`<li>`) to Markdown requires
    /// all `<li>` elements are translated to Markdown.
    pub markdown_translated: bool,
}

impl From<String> for HandlerResult {
    fn from(value: String) -> Self {
        HandlerResult {
            content: value,
            markdown_translated: true,
        }
    }
}

impl From<&str> for HandlerResult {
    fn from(value: &str) -> Self {
        HandlerResult {
            content: value.to_string(),
            markdown_translated: true,
        }
    }
}

impl HandlerResult {
    /// The result for an element written as HTML rather than translated. Its
    /// `markdown_translated` is false, so a container which needs every child
    /// in CommonMark can fall back to serializing itself.
    pub(crate) fn html(content: String) -> Self {
        HandlerResult {
            content,
            markdown_translated: false,
        }
    }
}

/// Conversion lifecycle event types.
///
/// # Example
///
/// ```
/// use htmd::element_handler::EventTypes;
///
/// let types = EventTypes::NONE;
/// let types = EventTypes::DOC_ENTER;
/// let types = EventTypes::DOC_ENTER | EventTypes::ELEMENT_ENTER;
/// let types = EventTypes::ALL - EventTypes::DOC_ENTER;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EventTypes(u8);

impl EventTypes {
    pub const NONE: Self = Self(0);
    pub const DOC_ENTER: Self = Self(1 << 0);
    pub const DOC_LEAVE: Self = Self(1 << 1);
    pub const ELEMENT_ENTER: Self = Self(1 << 2);
    pub const ELEMENT_LEAVE: Self = Self(1 << 3);

    pub const DOC_EVENTS: Self = Self(Self::DOC_ENTER.0 | Self::DOC_LEAVE.0);
    pub const ELEMENT_EVENTS: Self = Self(Self::ELEMENT_ENTER.0 | Self::ELEMENT_LEAVE.0);
    pub const ALL: Self = Self(Self::DOC_EVENTS.0 | Self::ELEMENT_EVENTS.0);

    #[inline(always)]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for EventTypes {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::Sub for EventTypes {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

/// A declarative, passive event subscription for an [`ElementHandler`].
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EventSubscription {
    /// The types of events to be notified.
    pub events: EventTypes,
    /// Specific tag names to receive element events for.
    /// If empty, no element events are received.
    pub tags: &'static [&'static str],
}

impl EventSubscription {
    pub const NONE: Self = Self {
        events: EventTypes::NONE,
        tags: &[],
    };
}

/// Trait for handling the conversion of a specific HTML element to Markdown.
pub trait ElementHandler: Send + Sync {
    /// Append additional content to the end of the converted Markdown.
    fn append(&self) -> Option<String> {
        None
    }

    /// Handle the conversion of an element.
    fn handle(&self, handlers: &dyn Handlers, element: Element) -> Option<HandlerResult>;

    /// Declare passive event subscriptions.
    ///
    /// The default implementation returns [`EventSubscription::NONE`], meaning no events
    /// are received and conversion runs with no event overhead.
    fn event_subscription(&self, _options: &Options) -> EventSubscription {
        EventSubscription::NONE
    }

    /// Called when starting a document conversion.
    fn on_doc_enter(&self) {}

    /// Called when finishing a document conversion.
    fn on_doc_leave(&self) {}

    /// Called before an element matching the subscribed tag names is processed.
    fn on_element_enter(&self, _element: &Element) {}

    /// Called after an element matching the subscribed tag names has been processed.
    fn on_element_leave(&self, _element: &Element, _result: Option<&HandlerResult>) {}
}

impl<F> ElementHandler for F
where
    F: (Fn(&dyn Handlers, Element) -> Option<HandlerResult>) + Send + Sync,
{
    fn handle(&self, handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
        self(handlers, element)
    }
}

#[derive(Default)]
pub(crate) struct TagEntry {
    pub(crate) handler_indices: Vec<usize>,
    pub(crate) enter_listeners: Vec<usize>,
    pub(crate) leave_listeners: Vec<usize>,
}

/// Builtin element handlers
pub(crate) struct ElementHandlers {
    pub(crate) handlers: Vec<Box<dyn ElementHandler>>,
    pub(crate) tag_entries: HashMap<String, TagEntry>,
    pub(crate) doc_enter_listeners: Vec<usize>,
    pub(crate) doc_leave_listeners: Vec<usize>,
    pub(crate) has_element_listeners: bool,
    pub(crate) can_passthrough_span: bool,
    pub(crate) options: Options,
}

impl ElementHandlers {
    fn walk_children_with(
        &self,
        node: &Rc<Node>,
        is_block: bool,
        context: Context,
    ) -> HandlerResult {
        let mut output = String::new();
        let markdown_translated =
            crate::dom_walker::walk_children(node, &mut output, self, is_block, context);
        HandlerResult {
            content: output,
            markdown_translated,
        }
    }

    pub(crate) fn has_tag_handler(&self, tag: &str) -> bool {
        self.tag_entries
            .get(tag)
            .is_some_and(|entry| !entry.handler_indices.is_empty())
    }

    pub(crate) fn has_tag_handler_or_listeners(&self, tag: &str) -> bool {
        let Some(entry) = self.tag_entries.get(tag) else {
            return false;
        };
        !entry.handler_indices.is_empty()
            || (self.has_element_listeners
                && (!entry.enter_listeners.is_empty() || !entry.leave_listeners.is_empty()))
    }

    pub(crate) fn tag_handler_count(&self, tag: &str) -> usize {
        self.tag_entries
            .get(tag)
            .map_or(0, |entry| entry.handler_indices.len())
    }

    #[inline(always)]
    pub(crate) fn emit_doc_enter(&self) {
        for &idx in &self.doc_enter_listeners {
            self.handlers[idx].on_doc_enter();
        }
    }

    #[inline(always)]
    pub(crate) fn emit_doc_leave(&self) {
        for &idx in &self.doc_leave_listeners {
            self.handlers[idx].on_doc_leave();
        }
    }

    pub(crate) fn rebuild_subscriptions(&mut self) {
        self.doc_enter_listeners.clear();
        self.doc_leave_listeners.clear();
        for entry in self.tag_entries.values_mut() {
            entry.enter_listeners.clear();
            entry.leave_listeners.clear();
        }

        let mut has_element_listeners = false;
        for (idx, handler) in self.handlers.iter().enumerate() {
            let sub = handler.event_subscription(&self.options);
            if sub.events.contains(EventTypes::DOC_ENTER) {
                self.doc_enter_listeners.push(idx);
            }
            if sub.events.contains(EventTypes::DOC_LEAVE) {
                self.doc_leave_listeners.push(idx);
            }
            if sub.events.contains(EventTypes::ELEMENT_ENTER) {
                for &tag in sub.tags {
                    let entry = self.tag_entries.entry(tag.to_owned()).or_default();
                    entry.enter_listeners.push(idx);
                    has_element_listeners = true;
                }
            }
            if sub.events.contains(EventTypes::ELEMENT_LEAVE) {
                for &tag in sub.tags {
                    let entry = self.tag_entries.entry(tag.to_owned()).or_default();
                    entry.leave_listeners.push(idx);
                    has_element_listeners = true;
                }
            }
        }
        self.has_element_listeners = has_element_listeners;
        self.can_passthrough_span = self.options.translation_mode == TranslationMode::Pure
            && self.tag_handler_count("span") == 1
            && self.tag_entries.get("span").is_none_or(|entry| {
                entry.enter_listeners.is_empty() && entry.leave_listeners.is_empty()
            });
    }

    pub fn new(options: Options) -> Self {
        let mut handlers = Self {
            handlers: Vec::new(),
            tag_entries: HashMap::new(),
            doc_enter_listeners: Vec::new(),
            doc_leave_listeners: Vec::new(),
            has_element_listeners: false,
            can_passthrough_span: false,
            options,
        };

        // img
        handlers.add_handler(vec!["img"], img_handler);

        // a
        handlers.add_handler(vec!["a"], AnchorElementHandler::new());

        // list
        handlers.add_handler(vec!["ol", "ul"], list_handler);

        // li
        handlers.add_handler(vec!["li"], list_item_handler);

        // quote
        handlers.add_handler(vec!["blockquote"], blockquote_handler);

        // code
        handlers.add_handler(vec!["code"], code_handler);

        // strong
        handlers.add_handler(vec!["strong", "b"], bold_handler);

        // italic
        handlers.add_handler(vec!["i", "em"], italic_handler);

        // headings
        handlers.add_handler(vec!["h1", "h2", "h3", "h4", "h5", "h6"], headings_handler);

        // br
        handlers.add_handler(vec!["br"], br_handler);

        // hr
        handlers.add_handler(vec!["hr"], hr_handler);

        // table
        handlers.add_handler(vec!["table"], table_handler);

        // td, th
        handlers.add_handler(vec!["td", "th"], td_th_handler);

        // tr
        handlers.add_handler(vec!["tr"], tr_handler);

        // thead, tbody
        handlers.add_handler(vec!["tbody", "thead"], table_section_handler);

        // caption
        handlers.add_handler(vec!["caption"], caption_handler);

        // p
        handlers.add_handler(vec!["p"], p_handler);

        // pre
        handlers.add_handler(vec!["pre"], pre_handler);

        // head, body
        handlers.add_handler(vec!["head", "body"], head_body_handler);

        // html
        handlers.add_handler(vec!["html"], html_handler);

        handlers.add_handler(vec!["span"], span_handler);

        let unhandled_block_elements = crate::util::node::BLOCK_ELEMENTS
            .iter()
            .filter(|tag| !handlers.has_tag_handler(tag))
            .copied()
            .collect();
        handlers.add_handler(unhandled_block_elements, block_handler);

        handlers
    }

    pub fn add_handler<Handler>(&mut self, tags: Vec<&str>, handler: Handler)
    where
        Handler: ElementHandler + 'static,
    {
        assert!(!tags.is_empty(), "tags cannot be empty.");
        let handler_idx = self.handlers.len();
        self.handlers.push(Box::new(handler));
        // Update tag entries
        for tag in tags {
            let entry = self.tag_entries.entry(tag.to_owned()).or_default();
            entry.handler_indices.push(handler_idx);
        }
        self.rebuild_subscriptions();
    }

    pub fn handle(
        &self,
        node: &Rc<Node>,
        tag: &str,
        attrs: &[Attribute],
        skipped_handlers: usize,
        context: Context,
    ) -> Option<HandlerResult> {
        let element = Element {
            node,
            tag,
            attrs,
            context,
            skipped_handlers,
        };

        let tag_entry = self.tag_entries.get(tag);

        if self.has_element_listeners
            && skipped_handlers == 0
            && let Some(entry) = tag_entry
        {
            for &idx in &entry.enter_listeners {
                self.handlers[idx].on_element_enter(&element);
            }
        }

        let result = match self.find_handler_in_entry(tag_entry, skipped_handlers) {
            Some(handler) => handler.handle(self, element),
            None => {
                if self.options.translation_mode == TranslationMode::Faithful {
                    Some(serialize_element_result(self, &element))
                } else {
                    // Default behavior: walk children and return their content
                    Some(self.walk_children(node, context))
                }
            }
        };

        if self.has_element_listeners
            && skipped_handlers == 0
            && let Some(entry) = tag_entry
        {
            for &idx in &entry.leave_listeners {
                self.handlers[idx].on_element_leave(&element, result.as_ref());
            }
        }

        result
    }

    fn find_handler_in_entry(
        &self,
        entry: Option<&TagEntry>,
        skipped_handlers: usize,
    ) -> Option<&dyn ElementHandler> {
        let entry = entry?;
        let idx = entry.handler_indices.iter().rev().nth(skipped_handlers)?;
        Some(self.handlers[*idx].as_ref())
    }
}

/// Provides access to the handlers for processing elements and nodes.
///
/// Handlers can use this to delegate to other handlers or recursively process child nodes.
pub trait Handlers {
    /// Skip the current handler and fall back to the previous handler (earlier in registration order).
    fn fallback(&self, element: Element) -> Option<HandlerResult>;

    /// Process a `markup5ever` node through the handlers. `context` is the
    /// [`Context`] the node appears in.
    fn handle(&self, node: &Rc<Node>, context: Context) -> Option<HandlerResult>;

    /// Walks children of a node and returns both content and markdown_translated
    /// status.
    fn walk_children(&self, node: &Rc<Node>, context: Context) -> HandlerResult;

    /// The content of [`Handlers::walk_children`]
    fn walk_children_content(&self, node: &Rc<Node>, context: Context) -> String {
        self.walk_children(node, context).content
    }

    /// Get the conversion options.
    fn options(&self) -> &Options;
}

impl Handlers for ElementHandlers {
    fn fallback(&self, element: Element) -> Option<HandlerResult> {
        self.handle(
            element.node,
            element.tag,
            element.attrs,
            element.skipped_handlers + 1,
            element.context,
        )
    }

    fn handle(&self, node: &Rc<Node>, context: Context) -> Option<HandlerResult> {
        let mut output = String::new();
        let markdown_translated = walk_node(node, &mut output, self, None, true, context);
        Some(HandlerResult {
            content: output,
            markdown_translated,
        })
    }

    fn walk_children(&self, node: &Rc<Node>, context: Context) -> HandlerResult {
        let tag = crate::util::node::get_node_tag_name(node);
        self.walk_children_with(
            node,
            tag.is_some_and(crate::util::node::is_block_element),
            context,
        )
    }

    fn options(&self) -> &Options {
        &self.options
    }
}

fn block_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    if handlers.options().translation_mode == TranslationMode::Pure {
        let content = handlers.walk_children_content(element.node, element.context);
        Some(frame_as_block(&content).into())
    } else {
        Some(serialize_element_result(handlers, &element))
    }
}

fn bold_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    emphasis_handler(handlers, element, "**")
}

fn italic_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    emphasis_handler(handlers, element, "*")
}
