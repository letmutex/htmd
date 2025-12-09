mod anchor;
mod blockquote;
mod br;
mod caption;
mod code;
mod element_util;
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
mod tbody;
mod td_th;
mod thead;
mod tr;

use crate::{
    dom_walker::walk_node,
    element_handler::{element_util::serialize_element, span::span_handler},
    options::{Options, TranslationMode},
    text_util::concat_strings,
};

use super::Element;
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
use std::{collections::HashMap, rc::Rc};
use table::table_handler;
use tbody::tbody_handler;
use td_th::td_th_handler;
use thead::thead_handler;
use tr::tr_handler;

/// The processing result of an `ElementHandler`.
pub struct HandlerResult {
    /// The converted content.
    pub content: String,
    /// See [`Element::markdown_translated`]
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

/// Trait for handling the conversion of a specific HTML element to Markdown.
pub trait ElementHandler: Send + Sync {
    /// Append additional content to the end of the converted Markdown.
    fn append(&self) -> Option<String> {
        None
    }

    /// Handle the conversion of an element.
    fn handle(&self, handlers: &dyn Handlers, element: Element) -> Option<HandlerResult>;
}

impl<F> ElementHandler for F
where
    F: (Fn(&dyn Handlers, Element) -> Option<HandlerResult>) + Send + Sync,
{
    fn handle(&self, handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
        self(handlers, element)
    }
}

/// Builtin element handlers
pub(crate) struct ElementHandlers {
    pub(crate) handlers: Vec<Box<dyn ElementHandler>>,
    pub(crate) tag_to_handler_indices: HashMap<String, Vec<usize>>,
    pub(crate) options: Options,
}

impl ElementHandlers {
    pub fn new(options: Options) -> Self {
        let mut handlers = Self {
            handlers: Vec::new(),
            tag_to_handler_indices: HashMap::new(),
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

        // tbody
        handlers.add_handler(vec!["tbody"], tbody_handler);

        // thead
        handlers.add_handler(vec!["thead"], thead_handler);

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

        // Other block elements. This is taken from the [CommonMark
        // spec](https://spec.commonmark.org/0.31.2/#html-blocks).
        handlers.add_handler(
            vec![
                "address",
                "article",
                "aside",
                "base",
                "basefont",
                "center",
                "col",
                "colgroup",
                "dd",
                "details",
                "dialog",
                "dir",
                "div",
                "dl",
                "dt",
                "fieldset",
                "figcaption",
                "figure",
                "footer",
                "form",
                "frame",
                "frameset",
                "header",
                "iframe",
                "legend",
                "link",
                "main",
                "menu",
                "menuitem",
                "nav",
                "noframes",
                "optgroup",
                "option",
                "param",
                "script",
                "search",
                "section",
                "style",
                "summary",
                "textarea",
                "tfoot",
                "title",
                "track",
            ],
            block_handler,
        );

        handlers
    }

    pub fn add_handler<Handler>(&mut self, tags: Vec<&str>, handler: Handler)
    where
        Handler: ElementHandler + 'static,
    {
        assert!(!tags.is_empty(), "tags cannot be empty.");
        let handler_idx = self.handlers.len();
        self.handlers.push(Box::new(handler));
        // Update tag to handler indices
        for tag in tags {
            let indices = self
                .tag_to_handler_indices
                .entry(tag.to_owned())
                .or_default();
            indices.push(handler_idx);
        }
    }

    pub fn handle(
        &self,
        node: &Rc<Node>,
        tag: &str,
        attrs: &[Attribute],
        markdown_translated: bool,
        skipped_handlers: usize,
    ) -> Option<HandlerResult> {
        match self.find_handler(tag, skipped_handlers) {
            Some(handler) => handler.handle(
                self,
                Element {
                    node,
                    tag,
                    attrs,
                    markdown_translated,
                    skipped_handlers,
                },
            ),
            None => {
                if self.options.translation_mode == TranslationMode::Faithful {
                    Some(HandlerResult {
                        content: serialize_element(
                            self,
                            &Element {
                                node,
                                tag,
                                attrs,
                                markdown_translated,
                                skipped_handlers: 0,
                            },
                        ),
                        markdown_translated: false,
                    })
                } else {
                    // Default behavior: walk children and return their content
                    Some(self.walk_children(node))
                }
            }
        }
    }

    fn find_handler(&self, tag: &str, skipped_handlers: usize) -> Option<&dyn ElementHandler> {
        let handler_indices = self.tag_to_handler_indices.get(tag)?;
        let idx = handler_indices.iter().rev().nth(skipped_handlers)?;
        Some(self.handlers[*idx].as_ref())
    }
}

/// Provides access to the handlers for processing elements and nodes.
///
/// Handlers can use this to delegate to other handlers or recursively process child nodes.
pub trait Handlers {
    /// Skip the current handler and fall back to the previous handler (earlier in registration order).
    fn fallback(&self, element: Element) -> Option<HandlerResult>;

    /// Process a `markup5ever` node through the handlers.
    fn handle(&self, node: &Rc<Node>) -> Option<HandlerResult>;

    /// Walks children of a node and returns both content and markdown_translated status.
    fn walk_children(&self, node: &Rc<Node>) -> HandlerResult;

    /// Get the conversion options.
    fn options(&self) -> &Options;
}

impl Handlers for ElementHandlers {
    fn fallback(&self, element: Element) -> Option<HandlerResult> {
        self.handle(
            element.node,
            element.tag,
            element.attrs,
            element.markdown_translated,
            element.skipped_handlers + 1,
        )
    }

    fn handle(&self, node: &Rc<Node>) -> Option<HandlerResult> {
        let mut buffer = Vec::new();
        let markdown_translated = walk_node(node, &mut buffer, self, None, true, false);
        let md = buffer.join("");
        Some(HandlerResult {
            content: md,
            markdown_translated,
        })
    }

    fn walk_children(&self, node: &Rc<Node>) -> HandlerResult {
        let mut buffer = Vec::new();
        let tag = crate::node_util::get_node_tag_name(node);
        let is_block = tag.is_some_and(crate::dom_walker::is_block_element);
        let is_pre = tag.is_some_and(|t| t == "pre" || t == "code") || is_inside_pre(node);
        let markdown_translated =
            crate::dom_walker::walk_children(node, &mut buffer, self, is_block, is_pre);
        HandlerResult {
            content: buffer.join(""),
            markdown_translated,
        }
    }

    fn options(&self) -> &Options {
        &self.options
    }
}

fn is_inside_pre(node: &Rc<Node>) -> bool {
    let mut current = crate::node_util::get_parent_node(node);
    while let Some(parent) = current {
        if let Some(tag) = crate::node_util::get_node_tag_name(&parent)
            && (tag == "pre" || tag == "code")
        {
            return true;
        }
        current = crate::node_util::get_parent_node(&parent);
    }
    false
}

fn block_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    if handlers.options().translation_mode == TranslationMode::Pure {
        let content = handlers.walk_children(element.node).content;
        let content = content.trim_matches('\n');
        Some(concat_strings!("\n\n", content, "\n\n").into())
    } else {
        Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        })
    }
}

fn bold_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    emphasis_handler(handlers, element, "**")
}

fn italic_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    emphasis_handler(handlers, element, "*")
}
