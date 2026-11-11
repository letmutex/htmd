use htmd::{HtmlToMarkdown, Node, convert};
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn convert_accepts_string_and_node_inputs() {
    let converter = HtmlToMarkdown::new();
    let tree = converter.html_to_tree("<p>Hi</p>").unwrap();
    let tree_ref = &tree;
    let tree_ref_ref: &&Node = &tree_ref;

    assert_eq!("Hi", convert("<p>Hi</p>").unwrap());
    let owned = String::from("<p>Hi</p>");
    let mut mutable = String::from("<p>Hi</p>");
    let boxed: Box<str> = "<p>Hi</p>".into();
    let arced: Arc<str> = Arc::from("<p>Hi</p>");
    let rced: Rc<str> = Rc::from("<p>Hi</p>");
    let cow: Cow<'_, str> = Cow::Borrowed("<p>Hi</p>");

    assert_eq!("Hi", converter.convert(owned.as_str()).unwrap());
    assert_eq!("Hi", converter.convert(&owned).unwrap());
    assert_eq!("Hi", converter.convert(mutable.as_mut_str()).unwrap());
    assert_eq!("Hi", converter.convert(&mut mutable).unwrap());
    assert_eq!("Hi", converter.convert(owned).unwrap());
    assert_eq!("Hi", converter.convert(boxed).unwrap());
    assert_eq!("Hi", converter.convert(arced).unwrap());
    assert_eq!("Hi", converter.convert(rced).unwrap());
    assert_eq!("Hi", converter.convert(cow).unwrap());
    assert_eq!("Hi", converter.convert(tree_ref).unwrap());
    assert_eq!("Hi", converter.convert(tree_ref_ref).unwrap());
    assert_eq!("Hi", converter.convert(tree).unwrap());
}

#[cfg(feature = "rcdom")]
#[test]
fn rcdom_conversions() {
    use html5ever::parse_document;
    use html5ever::tendril::TendrilSink;
    use markup5ever_rcdom::RcDom;

    let dom = parse_document(RcDom::default(), Default::default()).one("<p>Content</p>");
    assert_eq!("Content", convert(dom.document).unwrap());
}
