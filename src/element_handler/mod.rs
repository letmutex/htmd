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
    HtmlToMarkdown, dom_walker::is_block_element, options::TranslationMode,
    text_util::concat_strings,
};
use html5ever::{
    serialize::{SerializeOpts, TraversalScope, serialize},
    tendril::Tendril,
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
use markup5ever_rcdom::{Node, NodeData, SerializableHandle};
use p::p_handler;
use pre::pre_handler;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
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

    fn on_visit(
        &self,
        node: &Rc<Node>,
        html_to_markdown: &HtmlToMarkdown,
        tag: &str,
        attrs: &[Attribute],
        content: &str,
        markdown_translated: bool,
    ) -> (Option<String>, bool);
}

pub(crate) struct HandlerRule {
    tags: HashSet<String>,
    pub(crate) handler: Box<dyn ElementHandler>,
}

impl<F> ElementHandler for F
where
    F: (Fn(Element) -> (Option<String>, bool)) + Send + Sync,
{
    fn on_visit(
        &self,
        node: &Rc<Node>,
        html_to_markdown: &HtmlToMarkdown,
        tag: &str,
        attrs: &[Attribute],
        content: &str,
        markdown_translated: bool,
    ) -> (Option<String>, bool) {
        self(Element {
            node,
            tag,
            attrs,
            content,
            html_to_markdown,
            markdown_translated,
        })
    }
}

/// Builtin element handlers
pub(crate) struct ElementHandlers {
    pub(crate) rules: Vec<HandlerRule>,
}

impl ElementHandlers {
    pub fn new() -> Self {
        let mut handlers = Self { rules: Vec::new() };

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
}

impl ElementHandler for ElementHandlers {
    fn on_visit(
        &self,
        node: &Rc<Node>,
        html_to_markdown: &HtmlToMarkdown,
        tag: &str,
        attrs: &[Attribute],
        content: &str,
        markdown_translated: bool,
    ) -> (Option<String>, bool) {
        match self.rules.iter().rev().find(|rule| rule.tags.contains(tag)) {
            Some(rule) => rule.handler.on_visit(
                node,
                html_to_markdown,
                tag,
                attrs,
                content,
                markdown_translated,
            ),
            None => {
                if html_to_markdown.options.translation_mode == TranslationMode::Faithful {
                    (
                        Some(serialize_element(&Element {
                            node,
                            tag,
                            attrs,
                            content,
                            html_to_markdown,
                            markdown_translated,
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

fn block_handler(element: Element) -> (Option<String>, bool) {
    if element.html_to_markdown.options.translation_mode == TranslationMode::Pure {
        (Some(concat_strings!("\n\n", element.content, "\n\n")), true)
    } else {
        (Some(serialize_element(&element)), false)
    }
}

fn bold_handler(element: Element) -> (Option<String>, bool) {
    emphasis_handler(element, "**")
}

fn italic_handler(element: Element) -> (Option<String>, bool) {
    emphasis_handler(element, "*")
}

// Given a node (which is usually an element), serialize it (transform it back
// to HTML).
pub(crate) fn serialize_element(element: &Element) -> String {
    // If this is a block element, then serialize it and all its children.
    // Otherwise, serialize just this element, but use the current contents in
    // the place of children. This follows the Commonmark spec: [HTML
    // blocks](https://spec.commonmark.org/0.31.2/#html-blocks) contain only
    // HTML, not Markdown, while [raw HTML
    // inlines](https://spec.commonmark.org/0.31.2/#raw-html) contain Markdown.
    let is_be = is_block_element(element.tag);
    let node = if is_be {
        element.node.clone()
    } else {
        // Create a tree with just this element with one child: the text
        // collected so far.
        let NodeData::Element {
            name,
            attrs,
            template_contents,
            mathml_annotation_xml_integration_point,
        } = &element.node.data
        else {
            panic!("Not an element.");
        };
        let child = Rc::new(Node {
            parent: Cell::new(None),
            children: RefCell::new(vec![]),
            data: NodeData::Text {
                contents: RefCell::new(Tendril::from(element.content.to_string())),
            },
        });
        Rc::new(Node {
            parent: Cell::new(None),
            children: RefCell::new(vec![child]),
            data: NodeData::Element {
                name: name.clone(),
                attrs: attrs.clone(),
                template_contents: template_contents.clone(),
                mathml_annotation_xml_integration_point: *mathml_annotation_xml_integration_point,
            },
        })
    };

    let sh: SerializableHandle = SerializableHandle::from(node);
    let so = SerializeOpts {
        traversal_scope: TraversalScope::IncludeNode,
        ..Default::default()
    };
    let mut bytes = vec![];
    if let Err(err) = serialize(&mut bytes, &sh, so) {
        return err.to_string();
    }
    match String::from_utf8(bytes) {
        Ok(s) => {
            if is_be {
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
                concat_strings!("\n\n", result, "\n\n")
            } else {
                s
            }
        }
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
        if $element.html_to_markdown.options.translation_mode
            == $crate::options::TranslationMode::Faithful
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
