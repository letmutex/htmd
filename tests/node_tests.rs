use htmd::{Element, HtmlToMarkdown, Node, NodeType, element_handler::Handlers};
use std::hash::{DefaultHasher, Hash, Hasher};

#[test]
fn node_inspection_and_traversal() {
    let converter = HtmlToMarkdown::new();
    let tree = converter
        .html_to_tree("<!DOCTYPE html><!--c--><div id=\"a\" data-k=\"v\">hello</div>")
        .unwrap();
    assert_eq!(NodeType::Document, tree.node_type());
    assert_eq!(None, tree.tag_name());
    assert_eq!(None, tree.text());
    assert_eq!(None, tree.attr("id"));
    assert_eq!(0, tree.attrs().len());
    assert_eq!(0, tree.attrs().count());
    assert_eq!(None, tree.attrs().nth(1));
    assert_eq!(None, tree.attrs().nth_back(1));
    assert_eq!(None, tree.attrs().last());
    assert_eq!(None, tree.parent());
    assert_eq!(
        "<!DOCTYPE html><!--c--><html><head></head><body><div id=\"a\" data-k=\"v\">hello</div></body></html>",
        tree.html()
    );

    let children: Vec<Node> = tree.children().collect();
    assert_eq!(3, tree.children().count());
    assert_eq!(Some(children[1].clone()), tree.children().nth(1));
    assert_eq!(Some(children[1].clone()), tree.children().nth_back(1));
    assert_eq!(Some(children[2].clone()), tree.children().last());
    assert_eq!(None, tree.children().nth(3));
    assert_eq!(None, tree.children().nth_back(3));
    assert_eq!(NodeType::Doctype, children[0].node_type());
    assert_eq!("<!DOCTYPE html>", children[0].html());
    assert_eq!(NodeType::Comment, children[1].node_type());
    assert_eq!("<!--c-->", children[1].html());
    let html = &children[2];
    assert_eq!(Some("html"), html.tag_name());
    assert_eq!(Some(tree.clone()), html.parent());

    let div = tree
        .children()
        .find(|n| n.tag_name() == Some("html"))
        .unwrap()
        .children()
        .find(|n| n.tag_name() == Some("body"))
        .unwrap()
        .first_child()
        .unwrap();
    assert_eq!(NodeType::Element, div.node_type());
    assert_eq!(Some("div"), div.tag_name());
    assert_eq!(Some("a".to_string()), div.attr("id"));
    assert_eq!(None, div.attr("missing"));
    assert_eq!("<div id=\"a\" data-k=\"v\">hello</div>", div.html());
    assert_eq!(2, div.attrs().count());
    assert_eq!(
        Some(("data-k".to_string(), "v".to_string())),
        div.attrs().nth(1)
    );
    assert_eq!(
        Some(("id".to_string(), "a".to_string())),
        div.attrs().nth_back(1)
    );
    assert_eq!(
        Some(("data-k".to_string(), "v".to_string())),
        div.attrs().last()
    );
    assert_eq!(
        vec![
            ("id".to_string(), "a".to_string()),
            ("data-k".to_string(), "v".to_string()),
        ],
        div.attrs().collect::<Vec<_>>()
    );
    assert_eq!(
        vec![
            ("data-k".to_string(), "v".to_string()),
            ("id".to_string(), "a".to_string()),
        ],
        div.attrs().rev().collect::<Vec<_>>()
    );

    let text = div.first_child().unwrap();
    assert_eq!(NodeType::Text, text.node_type());
    assert_eq!(Some("hello".to_string()), text.text());
    assert_eq!("hello", text.html());
    assert_eq!(Some(div.clone()), text.parent());
    assert_eq!(Some(text.clone()), div.children().next_back());
    assert_eq!(vec![text], div.children().rev().collect::<Vec<_>>());

    let hash_of = |node: &Node| {
        let mut hasher = DefaultHasher::new();
        node.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(hash_of(&div), hash_of(&div.clone()));
    assert!(format!("{div:?}").contains("Node::Element"));
}

#[test]
fn element_attr_and_attrs() {
    let converter = HtmlToMarkdown::builder()
        .add_handler(
            vec!["div"],
            |_handlers: &dyn Handlers, element: &Element| {
                assert_eq!(Some("a"), element.attr("id"));
                assert_eq!(Some("v"), element.attr("data-k"));
                assert_eq!(None, element.attr("missing"));
                assert_eq!(
                    vec![("id", "a"), ("data-k", "v")],
                    element.attrs().collect::<Vec<_>>()
                );
                assert_eq!(
                    vec![("data-k", "v"), ("id", "a")],
                    element.attrs().rev().collect::<Vec<_>>()
                );
                Some("handled".into())
            },
        )
        .build();

    assert_eq!(
        "handled",
        converter
            .convert("<div id=\"a\" data-k=\"v\">hello</div>")
            .unwrap()
    );
}
