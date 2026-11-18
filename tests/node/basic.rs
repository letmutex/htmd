use htmd::{
    HtmlToMarkdown,
    node::{Node, NodeType},
};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::hash::{DefaultHasher, Hash, Hasher};

#[test]
fn node_types_and_html_serialization() {
    let converter = HtmlToMarkdown::new();
    let parsed = converter
        .html_to_tree("<!DOCTYPE html><!--c--><div id=\"a\" data-k=\"v\">hello</div>")
        .unwrap();
    let tree = parsed.root();
    assert_eq!(NodeType::Document, tree.node_type());
    assert_eq!(None, tree.tag_name());
    assert!(tree.text().is_none());
    assert_eq!(None, tree.comment());
    assert_eq!(
        "<!DOCTYPE html><!--c--><html><head></head><body><div id=\"a\" data-k=\"v\">hello</div></body></html>",
        tree.html()
    );

    let tree_children = tree.children();
    let children: Vec<Node<'_>> = tree_children.iter().collect();
    assert_eq!(NodeType::Doctype, children[0].node_type());
    assert_eq!("<!DOCTYPE html>", children[0].html());
    assert_eq!(None, children[0].comment());

    assert_eq!(NodeType::Comment, children[1].node_type());
    assert_eq!("<!--c-->", children[1].html());
    assert_eq!(Some("c"), children[1].comment());

    let html = &children[2];
    assert_eq!(Some("html"), html.tag_name());

    let div = tree.find(|n| n.tag_name() == Some("div")).unwrap();
    assert_eq!(NodeType::Element, div.node_type());
    assert_eq!(Some("div"), div.tag_name());
    assert_eq!("<div id=\"a\" data-k=\"v\">hello</div>", div.html());

    let text = div.first_child().unwrap();
    assert_eq!(NodeType::Text, text.node_type());
    assert_eq!(Some("hello"), text.text().as_deref());
    assert_eq!("hello", text.text().unwrap().to_string());
    assert_eq!("hello", text.html());
    assert_eq!(None, text.comment());

    let hash_of = |node: &Node<'_>| {
        let mut hasher = DefaultHasher::new();
        node.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(hash_of(&div), hash_of(&div.clone()));
    assert_eq!("Node::Document", format!("{tree:?}"));
    assert_eq!(r#"Node::Doctype("html")"#, format!("{:?}", children[0]));
    assert_eq!(r#"Node::Comment("c")"#, format!("{:?}", children[1]));
    assert_eq!(r#"Node::Element("div")"#, format!("{div:?}"));
    assert_eq!(r#"Node::Text("hello")"#, format!("{text:?}"));
}

#[test]
fn node_opening_and_closing_tags() {
    let converter = HtmlToMarkdown::new();
    let parsed = converter
        .html_to_tree("<!DOCTYPE html><!--c--><div id=\"a\" data-k=\"v\">hello</div>")
        .unwrap();
    let tree = parsed.root();
    assert_eq!(None, tree.opening_tag());
    assert_eq!(None, tree.closing_tag());

    let tree_children = tree.children();
    let children: Vec<Node<'_>> = tree_children.iter().collect();
    assert_eq!(None, children[0].opening_tag());
    assert_eq!(None, children[0].closing_tag());
    assert_eq!(None, children[1].opening_tag());
    assert_eq!(None, children[1].closing_tag());
    assert_eq!(Some("<html>".to_string()), children[2].opening_tag());
    assert_eq!(Some("</html>".to_string()), children[2].closing_tag());

    let div = tree.find(|n| n.tag_name() == Some("div")).unwrap();
    assert_eq!(
        Some("<div id=\"a\" data-k=\"v\">".to_string()),
        div.opening_tag()
    );
    assert_eq!(Some("</div>".to_string()), div.closing_tag());

    let text = div.first_child().unwrap();
    assert_eq!(None, text.opening_tag());
    assert_eq!(None, text.closing_tag());

    let bool_tree = converter.html_to_tree("<input disabled>").unwrap();
    let input = bool_tree
        .root()
        .find(|n| n.tag_name() == Some("input"))
        .unwrap();
    assert_eq!(
        Some("<input disabled=\"\">".to_string()),
        input.opening_tag()
    );
    assert_eq!(Some(String::new()), input.closing_tag());

    let esc_tree = converter
        .html_to_tree("<br><img src=\"a&b\" alt=\"q\"><svg><path d=\"M0 0\"></path></svg>")
        .unwrap();
    let br = esc_tree
        .root()
        .find(|n| n.tag_name() == Some("br"))
        .unwrap();
    assert_eq!(Some("<br>".to_string()), br.opening_tag());
    assert_eq!(Some(String::new()), br.closing_tag());
    let img = esc_tree
        .root()
        .find(|n| n.tag_name() == Some("img"))
        .unwrap();
    assert_eq!(
        Some("<img src=\"a&amp;b\" alt=\"q\">".to_string()),
        img.opening_tag()
    );
    assert_eq!(Some(String::new()), img.closing_tag());
    let path = esc_tree
        .root()
        .find(|n| n.tag_name() == Some("path"))
        .unwrap();
    assert_eq!(Some("<path d=\"M0 0\">".to_string()), path.opening_tag());
    assert_eq!(Some("</path>".to_string()), path.closing_tag());

    let svg_source_tree = converter
        .html_to_tree("<svg><source></source></svg>")
        .unwrap();
    let svg_source = svg_source_tree
        .root()
        .find(|n| n.tag_name() == Some("source"))
        .unwrap();
    assert_eq!(Some("<source>".to_string()), svg_source.opening_tag());
    assert_eq!(Some("</source>".to_string()), svg_source.closing_tag());
}

#[test]
fn node_attrs_and_node_str() {
    let converter = HtmlToMarkdown::new();
    let parsed = converter
        .html_to_tree("<div id=\"a\" data-k=\"v\">hello</div>")
        .unwrap();
    let tree = parsed.root();
    assert!(tree.attr("id").is_none());
    assert!(tree.attrs().is_empty());
    assert_eq!(0, tree.attrs().len());
    assert_eq!(None, tree.attrs().get("id"));
    assert_eq!(0, tree.attrs().iter().count());
    assert_eq!(None, tree.attrs().iter().nth(1));
    assert_eq!(None, tree.attrs().iter().nth_back(1));
    assert_eq!(None, tree.attrs().iter().last());

    let div = tree.find(|n| n.tag_name() == Some("div")).unwrap();
    let id = div.attr("id").unwrap();
    assert_eq!(Some("a"), div.attr("id").as_deref());
    assert_eq!("a", &*id);
    assert_eq!(id, "a");
    assert!("a" == id);
    let expected = String::from("a");
    assert!(id == expected);
    assert!(expected == id);
    assert_eq!("a", format!("{id}"));
    let data_k = div.attr("data-k").unwrap();
    assert!(id < data_k);
    assert_eq!(Ordering::Less, id.cmp(&data_k));
    assert_eq!("a", Borrow::<str>::borrow(&id));
    assert_ne!(hash_of_str(&id), hash_of_str(&data_k));
    assert!(div.attr("missing").is_none());

    let attrs = div.attrs();
    assert_eq!(2, attrs.len());
    assert!(!attrs.is_empty());
    assert_eq!(Some("a"), attrs.get("id"));
    assert_eq!(Some("v"), attrs.get("data-k"));
    assert_eq!(None, attrs.get("missing"));
    let mut iter = attrs.iter();
    let iter_clone = iter.clone();
    assert_eq!(Some(("id", "a")), iter.next());
    assert_eq!(
        vec![("id", "a"), ("data-k", "v")],
        iter_clone.collect::<Vec<_>>()
    );
    assert_eq!(Some(("data-k", "v")), attrs.iter().nth(1));
    assert_eq!(Some(("id", "a")), attrs.iter().nth_back(1));
    assert_eq!(Some(("data-k", "v")), attrs.iter().last());
    assert_eq!(
        vec![("id", "a"), ("data-k", "v")],
        attrs.iter().collect::<Vec<_>>()
    );
    assert_eq!(
        vec![("id", "a"), ("data-k", "v")],
        (&attrs).into_iter().collect::<Vec<_>>()
    );
    assert_eq!(
        vec![("data-k", "v"), ("id", "a")],
        attrs.iter().rev().collect::<Vec<_>>()
    );
    assert_eq!(r#"{"id": "a", "data-k": "v"}"#, format!("{attrs:?}"));

    let head = tree.find(|n| n.tag_name() == Some("head")).unwrap();
    assert!(head.attrs().is_empty());
    assert_eq!(0, head.attrs().len());
    assert_eq!(None, head.attrs().get("id"));
    assert_eq!(0, head.attrs().iter().count());

    let bool_tree = converter.html_to_tree("<input disabled>").unwrap();
    let input = bool_tree
        .root()
        .find(|n| n.tag_name() == Some("input"))
        .unwrap();
    assert_eq!(Some(""), input.attr("disabled").as_deref());
    assert_eq!(Some(""), input.attrs().get("disabled"));
}

#[test]
fn node_tree_traversal_and_siblings() {
    let converter = HtmlToMarkdown::new();
    let parsed = converter
        .html_to_tree("<!DOCTYPE html><!--c--><div id=\"a\" data-k=\"v\">hello</div>")
        .unwrap();
    let tree = parsed.root();
    assert_eq!(None, tree.parent());
    assert_eq!(None, tree.prev_sibling());
    assert_eq!(None, tree.next_sibling());

    let tree_children = tree.children();
    let children: Vec<Node<'_>> = tree_children.iter().collect();
    assert_eq!(3, tree_children.len());
    assert!(!tree_children.is_empty());
    assert_eq!(Some(children[0].clone()), tree_children.first());
    assert_eq!(Some(children[1].clone()), tree_children.get(1));
    assert_eq!(Some(children[2].clone()), tree_children.last());
    assert_eq!(None, tree_children.get(3));
    assert_eq!(Some(children[0].clone()), tree.first_child());
    assert_eq!(Some(children[2].clone()), tree.last_child());
    assert_eq!(None, children[0].prev_sibling());
    assert_eq!(Some(children[1].clone()), children[0].next_sibling());
    assert_eq!(Some(children[0].clone()), children[1].prev_sibling());
    assert_eq!(Some(children[2].clone()), children[1].next_sibling());
    assert_eq!(Some(children[1].clone()), children[2].prev_sibling());
    assert_eq!(None, children[2].next_sibling());
    assert_eq!(Some(tree.clone()), children[2].parent());

    let div = tree.find(|n| n.tag_name() == Some("div")).unwrap();
    let text = div.first_child().unwrap();
    assert_eq!(Some(text.clone()), div.last_child());
    assert_eq!(None, text.first_child());
    assert_eq!(None, text.last_child());
    assert_eq!(Some(div.clone()), text.parent());
    assert_eq!(Some(text.clone()), div.children().last());
    assert_eq!(r#"[Node::Text("hello")]"#, format!("{:?}", div.children()));
    assert_eq!(
        vec![text.clone()],
        div.children().iter().rev().collect::<Vec<_>>()
    );
    assert_eq!(
        vec![text.clone()],
        (&div.children()).into_iter().collect::<Vec<_>>()
    );

    let mut visited = 0;
    let first_match = tree.find(|n| {
        visited += 1;
        matches!(n.tag_name(), Some("head" | "body"))
    });
    assert_eq!(Some("head"), first_match.as_ref().and_then(Node::tag_name));
    assert_eq!(5, visited);

    assert_eq!(Some(div.clone()), div.find(|n| n.tag_name() == Some("div")));
    assert_eq!(None, div.find(|n| n.tag_name() == Some("missing")));

    let sibling_tree = converter.html_to_tree("<br><img>").unwrap();
    let br = sibling_tree
        .root()
        .find(|n| n.tag_name() == Some("br"))
        .unwrap();
    let img = sibling_tree
        .root()
        .find(|n| n.tag_name() == Some("img"))
        .unwrap();
    assert_eq!(Some(br.clone()), img.prev_sibling());
    assert_eq!(Some(img), br.next_sibling());
}

fn hash_of_str<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn node_children_iter_methods() {
    let converter = HtmlToMarkdown::new();
    let parsed = converter
        .html_to_tree("<div><span>1</span><span>2</span><span>3</span></div>")
        .unwrap();
    let div = parsed.root().find(|n| n.tag_name() == Some("div")).unwrap();
    let children = div.children();

    assert_eq!(3, children.len());
    assert_eq!(3, children.iter().count());
    assert_eq!(
        Some("3"),
        children
            .iter()
            .last()
            .and_then(|n| n.first_child())
            .and_then(|t| t.text().map(|s| s.to_string()))
            .as_deref()
    );
    assert_eq!(
        Some("2"),
        children
            .iter()
            .nth(1)
            .and_then(|n| n.first_child())
            .and_then(|t| t.text().map(|s| s.to_string()))
            .as_deref()
    );
    assert_eq!(
        Some("2"),
        children
            .iter()
            .nth_back(1)
            .and_then(|n| n.first_child())
            .and_then(|t| t.text().map(|s| s.to_string()))
            .as_deref()
    );
    assert_eq!(None, children.iter().nth(3));
    assert_eq!(None, children.iter().nth_back(3));
}

#[test]
fn template_and_annotation_xml_nodes() {
    let converter = HtmlToMarkdown::new();
    let parsed = converter
        .html_to_tree(
            "<template><em>inside</em></template>\
             <math><annotation-xml encoding=\"text/html\"><p>html</p></annotation-xml></math>",
        )
        .unwrap();
    let root = parsed.root();

    let template = root.find(|n| n.tag_name() == Some("template")).unwrap();
    assert_eq!(NodeType::Element, template.node_type());
    assert!(template.children().is_empty());
    assert_eq!(None, template.first_child());
    assert_eq!(None, template.last_child());

    let annotation = root
        .find(|n| n.tag_name() == Some("annotation-xml"))
        .unwrap();
    let annotation_child = annotation.first_child().unwrap();
    assert_eq!(Some("p"), annotation_child.tag_name());
    assert_eq!("html", converter.convert(&parsed).unwrap());
}

#[test]
fn synthetic_text_element_inspection() {
    let converter = HtmlToMarkdown::builder()
        .add_handler(
            vec!["em"],
            |handlers: &dyn htmd::element_handler::Handlers,
             element: &htmd::element_handler::Element<'_>| {
                let node = element.node;
                assert_eq!(NodeType::Element, node.node_type());
                assert_eq!(Some("em"), node.tag_name());
                let parent = node.parent().unwrap();
                assert_eq!(Some("p"), parent.tag_name());
                assert!(node.text().is_none());
                assert_eq!("<em>ab</em>", node.html());
                assert_eq!(Some("<em>".to_string()), node.opening_tag());
                assert_eq!(Some("</em>".to_string()), node.closing_tag());
                assert_eq!(None, node.prev_sibling());
                assert_eq!(None, node.next_sibling());

                let children = node.children();
                assert_eq!(1, children.len());
                assert!(!children.is_empty());
                assert_eq!(1, children.iter().count());
                assert_eq!(
                    Some("ab"),
                    children
                        .get(0)
                        .and_then(|n| n.text().map(|t| t.to_string()))
                        .as_deref()
                );
                assert_eq!(None, children.get(1));
                assert_eq!(
                    Some("ab"),
                    children
                        .iter()
                        .last()
                        .and_then(|n| n.text().map(|t| t.to_string()))
                        .as_deref()
                );
                #[allow(clippy::iter_nth_zero)]
                let nth_zero = children.iter().nth(0);
                assert_eq!(
                    Some("ab"),
                    nth_zero
                        .and_then(|n| n.text().map(|t| t.to_string()))
                        .as_deref()
                );
                assert_eq!(
                    Some("ab"),
                    children
                        .iter()
                        .nth_back(0)
                        .and_then(|n| n.text().map(|t| t.to_string()))
                        .as_deref()
                );
                assert_eq!(None, children.iter().nth(1));
                assert_eq!(None, children.iter().nth_back(1));

                let text_child = node.first_child().unwrap();
                assert_eq!(Some(text_child.clone()), node.last_child());
                assert_eq!(NodeType::Text, text_child.node_type());
                assert!(text_child.children().is_empty());
                assert_eq!(None, text_child.first_child());
                assert_eq!(None, text_child.last_child());
                assert_eq!(
                    Some(text_child.clone()),
                    node.find(|n| n.node_type() == NodeType::Text)
                );

                handlers.fallback(element)
            },
        )
        .build();

    assert_eq!(
        "*ab*",
        converter.convert("<p><em>a</em><em>b</em></p>").unwrap()
    );
}
