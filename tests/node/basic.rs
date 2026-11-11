use htmd::{HtmlToMarkdown, Node, NodeType};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::hash::{DefaultHasher, Hash, Hasher};

#[test]
fn node_inspection_and_traversal() {
    let converter = HtmlToMarkdown::new();
    let tree = converter
        .html_to_tree("<!DOCTYPE html><!--c--><div id=\"a\" data-k=\"v\">hello</div>")
        .unwrap();
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

    let children: Vec<Node> = tree.children().iter().cloned().collect();
    assert_eq!(3, tree.children().len());
    assert!(!tree.children().is_empty());
    assert_eq!(Some(&children[0]), tree.children().first());
    assert_eq!(Some(&children[1]), tree.children().get(1));
    assert_eq!(Some(&children[2]), tree.children().last());
    assert_eq!(None, tree.children().get(3));
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
    assert_eq!(Some(&text), div.children().last());
    assert_eq!(r#"[Node::Text("hello")]"#, format!("{:?}", div.children()));
    assert_eq!(
        vec![text.clone()],
        div.children().iter().rev().cloned().collect::<Vec<_>>()
    );
    assert_eq!(
        vec![text.clone()],
        (&div.children()).into_iter().cloned().collect::<Vec<_>>()
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
    let input = bool_tree.find(|n| n.tag_name() == Some("input")).unwrap();
    assert_eq!(Some(""), input.attr("disabled").as_deref());
    assert_eq!(Some(""), input.attrs().get("disabled"));

    let hash_of = |node: &Node| {
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
