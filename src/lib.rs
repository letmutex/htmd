#[cfg(not(any(feature = "ego-tree", feature = "rcdom")))]
compile_error!("Enable at least one DOM backend feature: `ego-tree` or `rcdom`.");

mod dom;
mod dom_walker;
mod element;
pub mod element_handler;
mod html_block;
mod input;
mod node;
pub mod options;
pub(crate) mod util;

use dom_walker::walk_node;
pub use element::{Context, ContextKind, Element};
use element_handler::ElementHandlers;
pub use element_handler::{ElementHandler, EventSubscription, EventTypes};
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{ParseOpts, parse_document};
pub use input::ConvertInput;
use node::TreeInner;
pub use node::{
    Node, NodeAttrs, NodeAttrsIter, NodeChildren, NodeChildrenIter, NodeStr, NodeType, Tree,
};
use options::Options;
use util::node::is_inside_pre;

use crate::element_handler::Handlers;

/// Convert HTML or a parsed [`Tree`] / [`Node`] to Markdown.
///
/// Example:
///
/// ```
/// use htmd::convert;
///
/// let md = convert("<h1>Hello</h1>").unwrap();
/// assert_eq!("# Hello", md);
/// ```
pub fn convert<I: ConvertInput>(input: I) -> std::io::Result<String> {
    HtmlToMarkdown::new().convert(input)
}

/// The html-to-markdown converter.
///
/// # Example
/// ```
/// use htmd::{Element, HtmlToMarkdown};
///
/// // One-liner
/// let md = HtmlToMarkdown::new().convert("<h1>Hello</h1>").unwrap();
/// assert_eq!("# Hello", md);
///
/// // Or use the builder pattern
/// let converter = HtmlToMarkdown::builder()
///     .skip_tags(vec!["img"])
///     .build();
/// let md = converter.convert("<img src=\"https://example.com\">").unwrap();
/// // img is ignored
/// assert_eq!("", md);
/// ```
pub struct HtmlToMarkdown {
    handlers: ElementHandlers,
    scripting_enabled: bool,
}

impl Default for HtmlToMarkdown {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlToMarkdown {
    /// Create a new converter.
    pub fn new() -> Self {
        let options = Options::default();
        let handlers = ElementHandlers::new(options);
        Self {
            handlers,
            scripting_enabled: true,
        }
    }

    pub(crate) fn from_params(handlers: ElementHandlers, scripting_enabled: bool) -> Self {
        Self {
            handlers,
            scripting_enabled,
        }
    }

    /// Create a new [HtmlToMarkdownBuilder].
    pub fn builder() -> HtmlToMarkdownBuilder {
        HtmlToMarkdownBuilder::new()
    }

    /// Parse HTML into a DOM [`Tree`].
    pub fn html_to_tree(&self, html: &str) -> std::io::Result<Tree> {
        // Feed the whole UTF-8 string to the parser in one shot. This avoids
        // the 4 KiB chunked `read_from` path and the UTF-8 lossy decoder layer
        // used by `from_utf8()`, both of which add overhead when the input is
        // already a valid `&str`.
        let opts = ParseOpts {
            tree_builder: TreeBuilderOpts {
                scripting_enabled: self.scripting_enabled,
                ..Default::default()
            },
            ..Default::default()
        };

        #[cfg(feature = "ego-tree")]
        let inner =
            TreeInner::Ego(parse_document(dom::ego_tree::EgoTreeSink::default(), opts).one(html));
        #[cfg(all(not(feature = "ego-tree"), feature = "rcdom"))]
        let inner = TreeInner::Rc(
            parse_document(markup5ever_rcdom::RcDom::default(), opts)
                .one(html)
                .document,
        );

        Ok(Tree(inner))
    }

    pub(crate) fn tree_to_markdown(&self, root: &Node<'_>) -> String {
        self.handlers.emit_doc_enter();
        let mut content = String::new();

        walk_node(
            root,
            &mut content,
            &self.handlers,
            None,
            true,
            Context {
                kind: ContextKind::Block,
                literal: is_inside_pre(root),
            },
        );

        // Trim the body's trailing newlines before the appends below, whose
        // content opens with a blank line separating it from the body.
        content.truncate(content.trim_end_matches('\n').len());

        for handler in &self.handlers.handlers {
            let Some(append_content) = handler.append() else {
                continue;
            };
            content.push_str(&append_content);
        }

        // Trim the document's edges in place instead of allocating a copy. The
        // leading trim runs after the appends, not before: with an empty body,
        // the first append's separating blank line would otherwise lead the
        // document.
        content.truncate(content.trim_end_matches('\n').len());
        let start = content.len() - content.trim_start_matches('\n').len();
        if start > 0 {
            content.drain(..start);
        }

        self.handlers.emit_doc_leave();
        content
    }

    /// Convert HTML or a parsed [`Tree`] / [`Node`] to Markdown.
    pub fn convert<I: ConvertInput>(&self, input: I) -> std::io::Result<String> {
        input.convert_with(self)
    }
}

/// The [HtmlToMarkdown] builder for advanced configurations.
pub struct HtmlToMarkdownBuilder {
    handlers: ElementHandlers,
    scripting_enabled: bool,
}

impl Default for HtmlToMarkdownBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlToMarkdownBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        let options = Options::default();
        let handlers = ElementHandlers::new(options);
        Self {
            handlers,
            scripting_enabled: true,
        }
    }

    /// Set converting options.
    pub fn options(mut self, options: Options) -> Self {
        self.handlers.options = options;
        self.handlers.rebuild_subscriptions();
        self
    }

    /// Skip a group of tags when converting.
    pub fn skip_tags(self, tags: Vec<&str>) -> Self {
        self.add_handler(tags, |_: &dyn Handlers, _: &Element| None)
    }

    /// Apply a custom element handler for a group of tags.
    ///
    /// # Example
    ///
    /// ```
    /// use htmd::{Element, HtmlToMarkdownBuilder, element_handler::Handlers};
    ///
    /// let mut handlers = HtmlToMarkdownBuilder::new()
    ///    .add_handler(vec!["img"], |_handlers: &dyn Handlers, _: &Element| {
    ///        // Skip the img tag when converting.
    ///        None
    ///    })
    ///    .add_handler(vec!["video"], |_handlers: &dyn Handlers, element: &Element| {
    ///        // Handle the video tag.
    ///        todo!("Return some text to represent this video element.")
    ///    });
    /// ```
    pub fn add_handler<Handler>(mut self, tags: Vec<&str>, handler: Handler) -> Self
    where
        Handler: ElementHandler + 'static,
    {
        self.handlers.add_handler(tags, handler);
        self
    }

    /// Whether `<noscript>` tags treat their content as raw text instead of HTML.
    /// When `false`, the content of `<noscript>` tags is parsed as normal DOM.
    pub fn scripting_enabled(mut self, enabled: bool) -> Self {
        self.scripting_enabled = enabled;
        self
    }

    /// Create a new [HtmlToMarkdown].
    pub fn build(self) -> HtmlToMarkdown {
        HtmlToMarkdown::from_params(self.handlers, self.scripting_enabled)
    }
}
