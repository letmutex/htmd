use htmd::{HtmlToMarkdown, Node, NodeType};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::hash::{DefaultHasher, Hash, Hasher};

#[test]
fn node_inspection_and_traversal() {
    let converter = HtmlToMarkdown::new();
    let parsed = converter
        .html_to_tree("<!DOCTYPE html><!--c--><div id=\"a\" data-k=\"v\">hello</div>")
        .unwrap();
    let tree = parsed.root();
    assert_eq!(NodeType::Document, tree.node_type());
    assert_eq!(None, tree.tag_name());
    assert!(tree.text().is_none());
    assert!(tree.attr("id").is_none());
    assert!(tree.attrs().is_empty());
    assert_eq!(0, tree.attrs().len());
    assert_eq!(None, tree.attrs().get("id"));
    assert_eq!(0, tree.attrs().iter().count());
    assert_eq!(None, tree.attrs().iter().nth(1));
    assert_eq!(None, tree.attrs().iter().nth_back(1));
    assert_eq!(None, tree.attrs().iter().last());
    assert_eq!(None, tree.parent());
    assert_eq!(
        "<!DOCTYPE html><!--c--><html><head></head><body><div id=\"a\" data-k=\"v\">hello</div></body></html>",
        tree.html()
    );

    let tree_children = tree.children();
    let children: Vec<Node<'_>> = tree_children.iter().collect();
    assert_eq!(3, tree_children.len());
    assert!(!tree_children.is_empty());
    assert_eq!(Some(children[0].clone()), tree_children.first());
    assert_eq!(Some(children[1].clone()), tree_children.get(1));
    assert_eq!(Some(children[2].clone()), tree_children.last());
    assert_eq!(None, tree_children.get(3));
    assert_eq!(Some(children[0].clone()), tree.first_child());

    assert_eq!(NodeType::Doctype, children[0].node_type());
    assert_eq!("<!DOCTYPE html>", children[0].html());
    assert_eq!(NodeType::Comment, children[1].node_type());
    assert_eq!("<!--c-->", children[1].html());
    let html = &children[2];
    assert_eq!(Some("html"), html.tag_name());
    assert_eq!(Some(tree.clone()), html.parent());

    let div = tree.find(|n| n.tag_name() == Some("div")).unwrap();
    assert_eq!(NodeType::Element, div.node_type());
    assert_eq!(Some("div"), div.tag_name());
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
    assert_eq!("<div id=\"a\" data-k=\"v\">hello</div>", div.html());

    {
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
    }

    let head = tree.find(|n| n.tag_name() == Some("head")).unwrap();
    assert!(head.attrs().is_empty());
    assert_eq!(0, head.attrs().len());
    assert_eq!(None, head.attrs().get("id"));
    assert_eq!(0, head.attrs().iter().count());

    let text = div.first_child().unwrap();
    assert_eq!(NodeType::Text, text.node_type());
    assert_eq!(Some("hello"), text.text().as_deref());
    assert_eq!("hello", text.text().unwrap().to_string());
    assert_eq!("hello", text.html());
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

    let bool_tree = converter.html_to_tree("<input disabled>").unwrap();
    let input = bool_tree
        .root()
        .find(|n| n.tag_name() == Some("input"))
        .unwrap();
    assert_eq!(Some(""), input.attr("disabled").as_deref());
    assert_eq!(Some(""), input.attrs().get("disabled"));

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
            |handlers: &dyn htmd::element_handler::Handlers, element: &htmd::Element<'_>| {
                let node = element.node;
                assert_eq!(NodeType::Element, node.node_type());
                assert_eq!(Some("em"), node.tag_name());
                let parent = node.parent().unwrap();
                assert_eq!(Some("p"), parent.tag_name());
                assert!(node.text().is_none());
                assert_eq!("<em>ab</em>", node.html());

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
                assert_eq!(NodeType::Text, text_child.node_type());
                assert!(text_child.children().is_empty());
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
