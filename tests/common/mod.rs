// Shared code for all integration tests.
use htmd::{
    ConvertInput, HtmlToMarkdown,
    options::{HeadingStyle, Options, TranslationMode},
};
use pulldown_cmark::{Options as CommonMarkOptions, Parser};

// By default, use the faithful translation mode, which is more stringent.
pub fn convert_faithful(input: impl ConvertInput) -> std::io::Result<String> {
    convert_faithful_options(input, Options::default())
}

// `convert_faithful` with the given options. The translation mode is
// overridden, so callers vary only the options they care about.
pub fn convert_faithful_options(
    input: impl ConvertInput,
    options: Options,
) -> std::io::Result<String> {
    HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            ..options
        })
        .build()
        .convert(input)
}

// `convert_faithful` with setext headings, the style whose h1 and h2 leave
// their content at the start of a line.
pub fn convert_faithful_setext(html: &str) -> std::io::Result<String> {
    convert_faithful_options(
        html,
        Options {
            heading_style: HeadingStyle::Setex,
            ..Default::default()
        },
    )
}

// The default translation mode, which drops what CommonMark cannot spell.
pub fn convert_pure(input: impl ConvertInput) -> std::io::Result<String> {
    HtmlToMarkdown::builder().build().convert(input)
}

// Takes `html` back to HTML the long way round: `convert_faithful` writes the
// Markdown, and pulldown-cmark reads that Markdown back.
//
// What comes back is HTML *source*, not a DOM, so a character reference in it
// is decoded only later, by whatever HTML parser reads the result. That is why
// several callers still assert a `&#10;`: it decodes to the line ending it
// replaced, so the trip is faithful even though the strings differ. Where a
// trip loses something instead, the assertion says what.
pub fn round_trip(html: &str) -> String {
    render_markdown(&convert_faithful(html).unwrap(), CommonMarkOptions::empty())
}

// The second half of a round trip: the CommonMark of the first half read back
// as HTML.
pub fn render_markdown(markdown: &str, options: CommonMarkOptions) -> String {
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, Parser::new_ext(markdown, options));
    html_output
}

// Builders for a tree handed to `HtmlToMarkdown::convert` directly, which can
// hold what the HTML parser never produces.
#[cfg(feature = "rcdom")]
pub mod tree {
    use html5ever::{LocalName, Prefix, QualName, ns};
    use markup5ever_rcdom::{Node, NodeData};
    use std::{cell::RefCell, rc::Rc};

    pub fn document(children: Vec<Rc<Node>>) -> Rc<Node> {
        with_children(Node::new(NodeData::Document), children)
    }

    pub fn element(tag: &str, children: Vec<Rc<Node>>) -> Rc<Node> {
        element_named(
            QualName::new(None, ns!(html), LocalName::from(tag)),
            children,
        )
    }

    // An HTML element whose name carries `prefix`, which the parser never gives
    // one.
    pub fn prefixed_element(prefix: &str, tag: &str, children: Vec<Rc<Node>>) -> Rc<Node> {
        element_named(
            QualName::new(Some(Prefix::from(prefix)), ns!(html), LocalName::from(tag)),
            children,
        )
    }

    fn element_named(name: QualName, children: Vec<Rc<Node>>) -> Rc<Node> {
        let node = Node::new(NodeData::Element {
            name,
            attrs: RefCell::new(Vec::new()),
            template_contents: RefCell::new(None),
            mathml_annotation_xml_integration_point: false,
        });
        with_children(node, children)
    }

    pub fn text(contents: &str) -> Rc<Node> {
        Node::new(NodeData::Text {
            contents: RefCell::new(contents.into()),
        })
    }

    fn with_children(parent: Rc<Node>, children: Vec<Rc<Node>>) -> Rc<Node> {
        for child in children {
            child.parent.set(Some(Rc::downgrade(&parent)));
            parent.children.borrow_mut().push(child);
        }
        parent
    }
}
