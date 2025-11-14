mod anchor;
mod blockquote;
mod br;
mod caption;
mod code;
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
mod table;
mod tbody;
mod td_th;
mod thead;
mod tr;

use crate::{
    dom_walker::{is_block_element, walk_node},
    options::{Options, TranslationMode},
    text_util::concat_strings,
};
use html5ever::serialize::{HtmlSerializer, SerializeOpts, Serializer, TraversalScope, serialize};

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
use markup5ever_rcdom::{Node, NodeData, SerializableHandle};
use p::p_handler;
use pre::pre_handler;
use std::{
    collections::HashSet,
    io::{self, Write},
    rc::Rc,
    sync::Arc,
};
use table::table_handler;
use tbody::tbody_handler;
use td_th::td_th_handler;
use thead::thead_handler;
use tr::tr_handler;

/// The DOM element handler.
pub trait ElementHandler: Send + Sync {
    fn append(&self) -> Option<String> {
        None
    }

    fn on_visit(&self, chain: &dyn Chain, element: Element) -> (Option<String>, bool);
}

pub(crate) struct HandlerRule {
    tags: HashSet<String>,
    pub(crate) handler: Box<dyn ElementHandler>,
}

impl<F> ElementHandler for F
where
    F: (Fn(&dyn Chain, Element) -> (Option<String>, bool)) + Send + Sync,
{
    fn on_visit(&self, chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
        self(chain, element)
    }
}

/// Builtin element handlers
pub(crate) struct ElementHandlers {
    pub(crate) rules: Vec<HandlerRule>,
    pub(crate) options: Arc<Options>,
}

impl ElementHandlers {
    pub fn new(options: Arc<Options>) -> Self {
        let mut handlers = Self {
            rules: Vec::new(),
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
        let handler = HandlerRule {
            tags: HashSet::from_iter(tags.iter().map(|tag| tag.to_string())),
            handler: Box::new(handler),
        };
        self.rules.push(handler);
    }

    pub fn handle(
        &self,
        node: &Rc<Node>,
        tag: &str,
        attrs: &[Attribute],
        content: &str,
        markdown_translated: bool,
        skipped_handlers: usize,
    ) -> (Option<String>, bool) {
        let rule = self
            .rules
            .iter()
            .filter(|rule| rule.tags.contains(tag))
            .rev()
            .skip(skipped_handlers)
            .next();
        match rule {
            Some(rule) => rule.handler.on_visit(
                self,
                Element {
                    node,
                    tag,
                    attrs,
                    content,
                    options: &self.options,
                    markdown_translated,
                    skipped_handlers,
                },
            ),
            None => {
                if self.options.translation_mode == TranslationMode::Faithful {
                    (
                        Some(serialize_element(&Element {
                            node,
                            tag,
                            attrs,
                            content,
                            options: &self.options,
                            markdown_translated,
                            skipped_handlers: 0,
                        })),
                        false,
                    )
                } else {
                    (Some(content.to_string()), true)
                }
            }
        }
    }
}

/// Provides access to the handler chain for processing elements and nodes.
///
/// Handlers can use this to delegate to other handlers or recursively process child nodes.
pub trait Chain {
    /// Skip the current handler and proceed to the previous handler (earlier in registration order).
    fn proceed(&self, element: Element) -> (Option<String>, bool);

    /// Process a `markup5ever` node through the handler chain.
    fn handle(&self, node: &Rc<Node>) -> (Option<String>, bool);
}

impl Chain for ElementHandlers {
    fn proceed(&self, element: Element) -> (Option<String>, bool) {
        self.handle(
            element.node,
            element.tag,
            element.attrs,
            element.content,
            element.markdown_translated,
            element.skipped_handlers + 1,
        )
    }

    fn handle(&self, node: &Rc<Node>) -> (Option<String>, bool) {
        let mut buffer = Vec::new();
        let markdown_translated = walk_node(node, &mut buffer, self, None, true, false);
        let md = buffer.join("");
        (Some(md), markdown_translated)
    }
}

fn block_handler(_chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
    if element.options.translation_mode == TranslationMode::Pure {
        (Some(concat_strings!("\n\n", element.content, "\n\n")), true)
    } else {
        (Some(serialize_element(&element)), false)
    }
}

fn bold_handler(chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
    emphasis_handler(chain, element, "**")
}

fn italic_handler(chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
    emphasis_handler(chain, element, "*")
}

// Given a node (which must be an element), serialize it (transform it back
// to HTML).
pub(crate) fn serialize_element(element: &Element) -> String {
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
            ser.writer.write_all(element.content.as_bytes())?;
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
        // The element to translate.
        $element: expr,
        // The maximum number of attributes allowed for this element.
        $num_attrs_allowed: expr
    ) => {
        if $element.options.translation_mode == $crate::options::TranslationMode::Faithful
            && $element.attrs.len() > $num_attrs_allowed
        {
            return (
                Some($crate::element_handler::serialize_element(&$element)),
                // This was translated using HTML, not Markdown.
                false,
            );
        }
    };
}
