use indoc::indoc;
use pretty_assertions::assert_eq;

use crate::common::convert_faithful;

// Should allow inline markup inside tables. These come from https://github.github.com/gfm/.
#[test]
fn test_table_with_inlines() {
    let html = r#"
    <table>
        <thead>
            <tr>
                <th><code>Type</code></th>
                <th><em>Example</em></th>
            </tr>
        </thead>
        <tbody>
            <tr>
                <td>Backslash escapes</td>
                <td>*not emphasized*</td>
            </tr>
            <tr>
                <td>Entity and numeric character references</td>
                <td>&amp;</td>
            </tr>
            <tr>
                <td>Code spans</td>
                <td><code>code</code></td>
            </tr>
            <tr>
                <td>Emphasis and strong emphasis</td>
                <td><em>emphasis</em> <strong>strong</strong></td>
            </tr>
            <tr>
                <td>Links</td>
                <td><a href="/uri" title="title">link</a></td>
            </tr>
            <tr>
                <td>Images</td>
                <td><img src="/url" alt="foo" title="title"></td>
            </tr>
            <tr>
                <td>Raw HTML</td>
                <td><foo></foo></td>
            </tr>
        </tbody>
    </table>
    "#;

    let expected = r#"
| `Type`                                  | *Example*             |
| --------------------------------------- | --------------------- |
| Backslash escapes                       | \*not emphasized\*    |
| Entity and numeric character references | \&                    |
| Code spans                              | `code`                |
| Emphasis and strong emphasis            | *emphasis* **strong** |
| Links                                   | [link](/uri "title")  |
| Images                                  | ![foo](/url "title")  |
| Raw HTML                                | <foo></foo>           |
"#
    .trim();

    let markdown = convert_faithful(html).unwrap();
    let result = markdown.trim();
    assert_eq!(expected, result);
}

/// A cell's contents are parsed as inline content, so a block element in a
/// cell is written as a raw HTML inline and the table stays a table. See
/// the "Translating HTML nodes" section of `translation_approach.md`.
#[test]
fn test_table_block_cells() {
    assert_eq!(
        indoc!(
            r#"
            | a | <p>b</p> |
            | - | -------- |
            | c | d        |"#
        ),
        // This has a block (a paragraph) in the table headings.
        convert_faithful(indoc!(
            r#"
            <table>
                <thead>
                    <tr>
                        <th>a</th>
                        <th><p>b</p></th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>c</td>
                        <td>d</td>
                    </tr>
                </tbody>
            </table>
            "#
        ))
        .unwrap()
    );
}

/// The parser wraps a stray `<tr>` in a `<tbody>`, so a `<table>` holds one
/// as a direct child only in a tree built elsewhere and handed to the public
/// `HtmlToMarkdown::convert`. Reading such a row as the body row
/// the parser would have made of it keeps that entry point from panicking on
/// a tree it did not parse itself.
#[cfg(feature = "rcdom")]
#[test]
fn a_row_directly_under_a_table_is_read_as_a_body_row() {
    use htmd::options::{Options, TranslationMode};
    use html5ever::{LocalName, QualName, ns};
    use markup5ever_rcdom::{Node, NodeData};
    use std::{cell::RefCell, rc::Rc};

    fn append(parent: &Rc<Node>, child: Rc<Node>) {
        child.parent.set(Some(Rc::downgrade(parent)));
        parent.children.borrow_mut().push(child);
    }

    fn element(tag: &str, children: Vec<Rc<Node>>) -> Rc<Node> {
        let node = Node::new(NodeData::Element {
            name: QualName::new(None, ns!(html), LocalName::from(tag)),
            attrs: RefCell::new(Vec::new()),
            template_contents: RefCell::new(None),
            mathml_annotation_xml_integration_point: false,
        });
        for child in children {
            append(&node, child);
        }
        node
    }

    fn text(contents: &str) -> Rc<Node> {
        Node::new(NodeData::Text {
            contents: RefCell::new(contents.into()),
        })
    }

    let tree = Node::new(NodeData::Document);
    append(
        &tree,
        element(
            "table",
            vec![
                element("tr", vec![element("th", vec![text("A")])]),
                element("tr", vec![element("td", vec![text("c")])]),
            ],
        ),
    );

    let converter = htmd::HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build();

    assert_eq!("| A |\n| - |\n| c |", converter.convert(&tree).unwrap());
}
