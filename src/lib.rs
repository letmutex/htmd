mod dom_walker;
pub mod element_handler;
pub(crate) mod node_util;
pub mod options;
pub(crate) mod text_util;

use std::rc::Rc;

use dom_walker::walk_node;
use element_handler::{ElementHandler, ElementHandlers};
use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, Attribute};
use markup5ever_rcdom::{Node, RcDom};
use options::Options;

/// Convert HTML to Markdown.
///
/// Example:
///
/// ```
/// use htmd::convert;
///
/// let md = convert("<h1>Hello</h1>").unwrap();
/// assert_eq!("# Hello", md);
/// ```
pub fn convert(html: &str) -> Result<String, std::io::Error> {
    HtmlToMarkdown::new().convert(html)
}

/// The DOM element.
pub struct Element<'a> {
    /// The html5ever node of the element.
    pub node: &'a Rc<Node>,
    /// The tag name.
    pub tag: &'a str,
    /// The attribute list.
    pub attrs: &'a Vec<Attribute>,
    /// The content text, can be raw text or converted Markdown text.
    pub content: &'a str,
    /// Converter options.
    pub options: &'a Options,
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
    options: Options,
    handlers: ElementHandlers,
}

impl HtmlToMarkdown {
    /// Create a new converter.
    pub fn new() -> Self {
        Self {
            options: Options::default(),
            handlers: ElementHandlers::new(),
        }
    }

    pub(crate) fn from_params(options: Options, handlers: ElementHandlers) -> Self {
        Self { options, handlers }
    }

    /// Create a new [HtmlToMarkdownBuilder].
    pub fn builder() -> HtmlToMarkdownBuilder {
        HtmlToMarkdownBuilder::new()
    }

    /// Convert HTML to Markdown.
    pub fn convert(&self, html: &str) -> Result<String, std::io::Error> {
        let dom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut html.as_bytes())?;

        let mut buffer: Vec<String> = Vec::new();

        let handlers: Box<&dyn ElementHandler> = Box::new(&self.handlers);

        walk_node(
            &dom.document,
            None,
            &mut buffer,
            &handlers,
            &self.options,
            false,
            true,
        );

        let mut content = buffer.join("").trim_matches(|ch| ch == '\n').to_string();

        let mut append = String::new();
        for rule in &self.handlers.rules {
            let Some(append_content) = rule.handler.append() else {
                continue;
            };
            append.push_str(&append_content);
        }

        content.push_str(append.trim_end_matches(|ch| ch == '\n'));

        Ok(content)
    }
}

/// The [HtmlToMarkdown] builder for advanced configurations.
pub struct HtmlToMarkdownBuilder {
    options: Options,
    handlers: ElementHandlers,
}

impl HtmlToMarkdownBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            options: Options::default(),
            handlers: ElementHandlers::new(),
        }
    }

    /// Set converting options.
    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// Skip a group of tags when converting.
    pub fn skip_tags(self, tags: Vec<&str>) -> Self {
        self.add_handler(tags, |_: Element| None)
    }

    /// Apply a custom element handler for a group of tags.
    ///
    /// # Example
    ///
    /// ```
    /// use htmd::{Element, HtmlToMarkdownBuilder};
    ///
    /// let mut handlers = HtmlToMarkdownBuilder::new()
    ///    .add_handler(vec!["img"], |_: Element| {
    ///        // Skip the img tag when converting.
    ///        None
    ///    })
    ///    .add_handler(vec!["video"], |element: Element| {
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

    /// Create a new [HtmlToMarkdown].
    pub fn build(self) -> HtmlToMarkdown {
        HtmlToMarkdown::from_params(self.options, self.handlers)
    }
}
