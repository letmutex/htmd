mod common;
use common::convert_faithful;

#[cfg(test)]
mod table_tests_1 {
    use super::convert_faithful;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    fn longest_space_run(text: &str) -> usize {
        let mut longest = 0;
        let mut current = 0;

        for ch in text.chars() {
            if ch == ' ' {
                current += 1;
                longest = longest.max(current);
            } else {
                current = 0;
            }
        }

        longest
    }

    #[test]
    fn test_simple_table() {
        let html = r#"
        <table>
            <tr>
                <th>Header 1</th>
                <th>Header 2</th>
            </tr>
            <tr>
                <td>Cell 1</td>
                <td>Cell 2</td>
            </tr>
            <tr>
                <td>Cell 3</td>
                <td>Cell 4</td>
            </tr>
        </table>
        "#;

        let expected = r#"
| Header 1 | Header 2 |
| -------- | -------- |
| Cell 1   | Cell 2   |
| Cell 3   | Cell 4   |
"#
        .trim();

        let markdown = convert_faithful(html).unwrap();
        let result = markdown.trim();
        assert_eq!(expected, result);
    }

    #[test]
    fn table_rows_do_not_drop_cells_beyond_the_header_width() {
        let markdown = htmd::convert(
            r#"
            <table>
                <thead>
                    <tr><th>First</th><th>Second</th></tr>
                </thead>
                <tbody>
                    <tr><td>Alpha</td><td>Beta</td><td>Must survive</td></tr>
                </tbody>
            </table>
            "#,
        )
        .unwrap();

        assert_eq!(
            indoc!(
                r#"
                | First | Second |              |
                | ----- | ------ | ------------ |
                | Alpha | Beta   | Must survive |
                "#
            )
            .trim(),
            markdown
        );
    }

    #[test]
    fn malformed_table_foster_parenting_does_not_lose_visible_text() {
        let markdown =
            htmd::convert("<table>before<tr><th>heading</th></tr>after</table>").unwrap();

        assert_eq!(
            indoc!(
                r#"
                beforeafter

                | heading |
                | ------- |
                "#
            )
            .trim(),
            markdown
        );
    }

    #[test]
    fn test_table_with_thead_tbody() {
        let html = r#"
        <table>
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Age</th>
                    <th>Location</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>John</td>
                    <td>35</td>
                    <td>New York</td>
                </tr>
                <tr>
                    <td>Jane</td>
                    <td>28</td>
                    <td>San Francisco</td>
                </tr>
            </tbody>
        </table>
        "#;

        let expected = r#"
| Name | Age | Location      |
| ---- | --- | ------------- |
| John | 35  | New York      |
| Jane | 28  | San Francisco |
"#
        .trim();

        let markdown = convert_faithful(html).unwrap();
        let result = markdown.trim();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_table_with_thead_td_headers() {
        let html = r#"
        <table>
            <thead>
                <tr>
                    <td>Name</td>
                    <td>Age</td>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>John</td>
                    <td>35</td>
                </tr>
                <tr>
                    <td>Jane</td>
                    <td>28</td>
                </tr>
            </tbody>
        </table>
        "#;

        let expected = r#"
| Name | Age |
| ---- | --- |
| John | 35  |
| Jane | 28  |
"#
        .trim();

        let markdown = htmd::HtmlToMarkdown::new().convert(html).unwrap();
        let result = markdown.trim();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_table_with_caption() {
        let html = r#"
        <table>
            <caption>Sample Table</caption>
            <tbody>
                <tr>
                    <td>John</td>
                    <td>35</td>
                    <td>New York</td>
                </tr>
                <tr>
                    <td>Jane</td>
                    <td>28</td>
                    <td>San Francisco</td>
                </tr>
            </tbody>
        </table>
        "#;

        // A `<caption>` has no CommonMark spelling and is invalid outside a
        // `<table>`, so faithful mode serializes the whole table.
        let expected = html.trim();

        let markdown = convert_faithful(html).unwrap();
        let result = markdown.trim();
        assert_eq!(expected, result);
    }

    /// CommonMark has no caption: writing the content as a paragraph above the
    /// table reads as one but is not one, so only pure mode does it.
    #[test]
    fn test_table_caption_is_commonmark_only_in_pure_mode() {
        let html = concat!(
            "<table><caption>Sample Table</caption>",
            "<thead><tr><th>h</th></tr></thead>",
            "<tbody><tr><td>John</td></tr></tbody></table>"
        );

        assert_eq!(
            "Sample Table\n\n| h    |\n| ---- |\n| John |",
            htmd::HtmlToMarkdown::new().convert(html).unwrap()
        );
        // A `<caption>` is only valid inside a `<table>`, so the whole table is
        // serialized around it rather than the caption written on its own.
        assert_eq!(html, convert_faithful(html).unwrap());
    }

    /// A GFM table has no headerless form: the delimiter row which makes the
    /// block a table has to follow a header row.
    #[test]
    fn a_table_with_no_header_row_is_written_as_html() {
        let html = concat!(
            "<table><tbody><tr><td>a</td><td>b</td></tr>",
            "<tr><td>c</td><td>d</td></tr></tbody></table>"
        );
        assert_eq!(html, convert_faithful(html).unwrap());

        // A block element in a cell is a raw HTML inline and no reason to
        // serialize; the missing header row is the reason here.
        let block_cell = "<table><tbody><tr><td><p>a</p></td></tr></tbody></table>";
        assert_eq!(block_cell, convert_faithful(block_cell).unwrap());
    }

    /// Pure mode has no HTML to fall back on, so it writes an empty header row.
    /// A table is only built where the markup holds a `th` or a `thead`, hence
    /// the empty `<thead>` needed to reach this.
    #[test]
    fn pure_mode_writes_an_empty_header_row_for_a_headerless_table() {
        let html = "<table><thead></thead><tbody><tr><td>a</td></tr></tbody></table>";

        assert_eq!(
            "|   |\n| - |\n| a |",
            htmd::HtmlToMarkdown::new().convert(html).unwrap()
        );
        assert_eq!(html, convert_faithful(html).unwrap());
    }

    /// Asserts that faithful mode writes `html` back as it stands — the
    /// fallback for a table holding what a Markdown one cannot — and that pure
    /// mode, which has no fallback, writes `pure_markdown` instead.
    fn assert_html_fallback(html: &str, pure_markdown: &str) {
        assert_eq!(html, convert_faithful(html).unwrap());
        assert_eq!(
            pure_markdown,
            htmd::HtmlToMarkdown::new().convert(html).unwrap()
        );
    }

    /// A GFM table has one header row, so only the first `<tr>` of a `<thead>`
    /// reaches the Markdown; the rest would be dropped. Pure mode drops the
    /// second header row.
    #[test]
    fn extra_header_rows_are_written_as_html() {
        assert_html_fallback(
            concat!(
                "<table><thead><tr><th>A</th></tr><tr><th>B</th></tr></thead>",
                "<tbody><tr><td>c</td></tr></tbody></table>"
            ),
            "| A |\n| - |\n| c |",
        );
    }

    /// A GFM row is all header cells or all body cells, so a `<th>` among a body
    /// row's `<td>`s would be dropped. Pure mode drops it, which leaves the
    /// row's `<td>` in the first column.
    #[test]
    fn a_header_cell_in_a_body_row_is_written_as_html() {
        assert_html_fallback(
            concat!(
                "<table><tbody><tr><th>A</th><th>B</th></tr>",
                "<tr><th>r</th><td>c</td></tr></tbody></table>"
            ),
            "| A | B |\n| - | - |\n| c |   |",
        );
    }

    /// A `<colgroup>`, a `<script>` between rows, and a row holding no cell all
    /// reach the Markdown table as nothing at all.
    #[test]
    fn table_content_a_markdown_row_cannot_hold_is_written_as_html() {
        for html in [
            concat!(
                "<table><colgroup><col></colgroup><thead><tr><th>A</th></tr></thead>",
                "<tbody><tr><td>c</td></tr></tbody></table>"
            ),
            concat!(
                "<table><thead><tr><th>A</th></tr></thead>",
                "<tbody><script>x</script><tr><td>c</td></tr></tbody></table>"
            ),
            concat!(
                "<table><thead><tr><th>A</th></tr></thead>",
                "<tbody><tr><td>c</td></tr><tr></tr></tbody></table>"
            ),
        ] {
            assert_html_fallback(html, "| A |\n| - |\n| c |");
        }
    }

    /// The cells of a GFM header row are header cells, so a `<thead>` of `<td>`
    /// reaches the Markdown promoted to `<th>`. Pure mode takes the promotion.
    #[test]
    fn a_thead_of_data_cells_is_written_as_html() {
        assert_html_fallback(
            concat!(
                "<table><thead><tr><td>A</td><td>B</td></tr></thead>",
                "<tbody><tr><td>c</td><td>d</td></tr></tbody></table>"
            ),
            "| A | B |\n| - | - |\n| c | d |",
        );
    }

    /// A GFM table has one body, so a second `<tbody>`'s rows join the first and
    /// the split between them is lost. Pure mode takes the join.
    #[test]
    fn a_second_tbody_is_written_as_html() {
        assert_html_fallback(
            concat!(
                "<table><thead><tr><th>h</th></tr></thead>",
                "<tbody><tr><td>a</td></tr></tbody>",
                "<tbody><tr><td>b</td></tr></tbody></table>"
            ),
            "| h |\n| - |\n| a |\n| b |",
        );
    }

    /// A GFM table has one header row, so a second `<thead>` overwrites the
    /// header the first one gave and that header is lost. Pure mode takes the
    /// overwrite.
    #[test]
    fn a_second_thead_is_written_as_html() {
        assert_html_fallback(
            concat!(
                "<table><thead><tr><th>A</th></tr></thead>",
                "<thead><tr><th>B</th></tr></thead>",
                "<tbody><tr><td>c</td></tr></tbody></table>"
            ),
            "| B |\n| - |\n| c |",
        );
    }

    /// A GFM table's header row comes first, so a `<thead>` following body rows
    /// can only become one by moving ahead of the rows it followed. A `<tbody>`
    /// opening with a `<th>` row supplies the header just as a `<thead>` does,
    /// which leaves the `<thead>` after it the second of two. Pure mode takes
    /// the reordering.
    #[test]
    fn a_thead_after_a_body_row_is_written_as_html() {
        for html in [
            concat!(
                "<table><tbody><tr><td>c</td></tr></tbody>",
                "<thead><tr><th>B</th></tr></thead></table>"
            ),
            concat!(
                "<table><tbody><tr><th>A</th></tr><tr><td>c</td></tr></tbody>",
                "<thead><tr><th>B</th></tr></thead></table>"
            ),
        ] {
            assert_html_fallback(html, "| B |\n| - |\n| c |");
        }
    }

    /// A `<tbody>` holding no row reaches the Markdown as nothing at all, so the
    /// `<thead>` after it would move ahead of a body section the table no longer
    /// has. Pure mode takes the reordering, which leaves a table of a header
    /// alone.
    #[test]
    fn a_thead_after_an_empty_tbody_is_written_as_html() {
        assert_html_fallback(
            concat!(
                "<table><tbody></tbody>",
                "<thead><tr><th>B</th></tr></thead></table>"
            ),
            "| B |\n| - |",
        );
    }

    /// A GFM table's header row comes first, so a row of `<th>` following a body
    /// row can only become one by moving ahead of the rows it followed. Pure
    /// mode drops the row, as it drops a `<th>` among a body row's `<td>`s; the
    /// header it leaves empty is the one
    /// `pure_mode_writes_an_empty_header_row_for_a_headerless_table` covers.
    #[test]
    fn a_header_row_after_a_body_row_is_written_as_html() {
        assert_html_fallback(
            "<table><tbody><tr><td>1</td></tr><tr><th>H</th></tr></tbody></table>",
            "|   |\n| - |\n| 1 |",
        );
    }

    /// Every row of a GFM table holds the columns its header declares, so a row
    /// of a different width gains cells the HTML never held, or gives the table
    /// a header column the HTML never declared.
    #[test]
    fn a_row_wider_or_narrower_than_the_header_is_written_as_html() {
        assert_html_fallback(
            concat!(
                "<table><thead><tr><th>A</th></tr></thead>",
                "<tbody><tr><td>c</td><td>d</td></tr></tbody></table>"
            ),
            "| A |   |\n| - | - |\n| c | d |",
        );
        assert_html_fallback(
            concat!(
                "<table><thead><tr><th>A</th><th>B</th></tr></thead>",
                "<tbody><tr><td>c</td></tr></tbody></table>"
            ),
            "| A | B |\n| - | - |\n| c |   |",
        );
    }

    /// A Markdown table is built from its cells' content, which leaves a comment
    /// between them nowhere to go. Stray text is not the same case: the parser
    /// foster-parents it out of the table, ahead of the table's own output.
    #[test]
    fn a_comment_in_a_table_is_written_as_html() {
        for html in [
            concat!(
                "<table><!-- c --><thead><tr><th>A</th></tr></thead>",
                "<tbody><tr><td>c</td></tr></tbody></table>"
            ),
            concat!(
                "<table><thead><tr><th>A</th></tr></thead>",
                "<tbody><!-- c --><tr><td>c</td></tr></tbody></table>"
            ),
            concat!(
                "<table><thead><tr><th>A</th></tr></thead>",
                "<tbody><tr><!-- c --><td>c</td></tr></tbody></table>"
            ),
        ] {
            assert_html_fallback(html, "| A |\n| - |\n| c |");
        }

        let with_stray_text = concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<tbody>stray<tr><td>c</td></tr></tbody></table>"
        );
        assert_eq!(
            "stray\n\n| A |\n| - |\n| c |",
            convert_faithful(with_stray_text).unwrap()
        );
    }

    /// A `|` in a cell is escaped as `\|`, but a backslash escape written
    /// inside a tag stays the two characters it is. A `|` in a raw HTML
    /// inline's open tag is encoded as the character reference the HTML parser
    /// reading the result decodes back.
    #[test]
    fn a_pipe_in_an_open_tag_is_encoded() {
        let in_a_body_cell = concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<tbody><tr><td><span title=\"a|b\">c</span></td></tr></tbody></table>"
        );
        assert_eq!(
            concat!(
                "| A                               |\n",
                "| ------------------------------- |\n",
                "| <span title=\"a&#124;b\">c</span> |"
            ),
            convert_faithful(in_a_body_cell).unwrap()
        );

        let in_a_header_cell = concat!(
            "<table><thead><tr><th><span title=\"a|b\">c</span></th></tr></thead>",
            "<tbody><tr><td>c</td></tr></tbody></table>"
        );
        assert_eq!(
            concat!(
                "| <span title=\"a&#124;b\">c</span> |\n",
                "| ------------------------------- |\n",
                "| c                               |"
            ),
            convert_faithful(in_a_header_cell).unwrap()
        );

        // Pure mode drops the `<span>`, and the attribute with it.
        assert_eq!(
            "| A |\n| - |\n| c |",
            htmd::HtmlToMarkdown::new().convert(in_a_body_cell).unwrap()
        );
    }

    /// Neither escape for a `|` reaches inside a comment, so a comment holding
    /// one takes the whole table to HTML. A nested table's comment is written
    /// into the outer row too, so it takes the outer table with it.
    #[test]
    fn a_comment_holding_a_pipe_is_written_as_html() {
        // Pure mode drops every comment, so none of these reaches a row.
        assert_html_fallback(
            concat!(
                "<table><tbody><tr><th>A</th></tr>",
                "<tr><td><!-- a|b --></td></tr></tbody></table>"
            ),
            "| A |\n| - |\n|   |",
        );
        for html in [
            concat!(
                "<table><tbody><tr><th><!-- a|b --></th></tr>",
                "<tr><td>c</td></tr></tbody></table>"
            ),
            concat!(
                "<table><tbody><tr><th>A</th></tr><tr><td>",
                "<table><tbody><tr><td><!-- a|b --></td></tr></tbody></table>",
                "</td></tr></tbody></table>"
            ),
        ] {
            assert_eq!(html, convert_faithful(html).unwrap());
        }

        // A comment holding no `|` still translates, as
        // `a_comment_in_a_table_is_written_as_html` shows for one outside a
        // cell.
        let without_a_pipe = concat!(
            "<table><thead><tr><th>A</th></tr></thead>",
            "<tbody><tr><td>a<!-- c -->b</td></tr></tbody></table>"
        );
        assert_eq!(
            "| A            |\n| ------------ |\n| a<!-- c -->b |",
            convert_faithful(without_a_pipe).unwrap()
        );
    }

    /// A column whose every cell is empty measures zero, but a delimiter cell
    /// holding no `-` stops the row being a delimiter row. Every column is at
    /// least one wide so that the delimiter row stays aligned with the rows
    /// around it.
    #[test]
    fn an_empty_column_is_one_wide() {
        let only_column = concat!(
            "<table><thead><tr><th></th></tr></thead>",
            "<tbody><tr><td></td></tr></tbody></table>"
        );
        assert_eq!(
            "|   |\n| - |\n|   |",
            convert_faithful(only_column).unwrap()
        );

        let among_others = concat!(
            "<table><thead><tr><th></th><th>B</th></tr></thead>",
            "<tbody><tr><td></td><td>d</td></tr></tbody></table>"
        );
        assert_eq!(
            "|   | B |\n| - | - |\n|   | d |",
            convert_faithful(among_others).unwrap()
        );
    }

    /// A GFM table is a header row and the rows under it, so a table holding no
    /// row at all is not one. Pure mode, with no fallback, drops it.
    #[test]
    fn an_empty_table_is_written_as_html() {
        assert_html_fallback("<table></table>", "");
        assert_html_fallback("<table><tbody></tbody></table>", "");
        assert_html_fallback("<table><thead></thead></table>", "");
    }

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
| Entity and numeric character references | &                     |
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
    /// the "Translating HTML nodes" section of `unsupported_html.md`.
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

    #[test]
    fn test_hacker_news_layout_table_does_not_turn_into_giant_padded_rows() {
        let html = include_str!("../examples/page-to-markdown/html/Hacker News.html");

        let markdown = htmd::HtmlToMarkdown::new().convert(html).unwrap();

        assert!(
            markdown.contains("Apollo 8 astronaut William Anders ID'd in WA plane crash"),
            "expected converted output to retain visible story text"
        );
        assert!(
            markdown.lines().count() > 20,
            "expected Hacker News content to span many lines, got {} lines",
            markdown.lines().count()
        );
        assert!(
            longest_space_run(&markdown) < 200,
            "expected no pathological whitespace padding, got a run of {} spaces",
            longest_space_run(&markdown)
        );
    }

    #[test]
    fn test_headerless_table() {
        let html = r#"
        <table>
            <tr>
                <td>Alpha</td>
                <td>Beta</td>
            </tr>
            <tr>
                <td>Gamma</td>
                <td>Delta</td>
            </tr>
        </table>
        "#;

        let expected = "Alpha\n\nBeta\n\nGamma\n\nDelta";
        let markdown = htmd::HtmlToMarkdown::new().convert(html).unwrap();

        assert_eq!(expected, markdown);
    }

    #[test]
    fn test_headered_with_inner_headerless() {
        let html = r#"
        <table>
            <thead>
                <tr>
                    <th>Section</th>
                    <th>Details</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Alpha</td>
                    <td>
                        <table>
                            <tr>
                                <td>One</td>
                                <td>Two</td>
                            </tr>
                        </table>
                    </td>
                </tr>
            </tbody>
        </table>
        "#;

        let expected = indoc!(
            r#"
            | Section | Details  |
            | ------- | -------- |
            | Alpha   | One  Two |
            "#
        )
        .trim();
        let markdown = htmd::HtmlToMarkdown::new().convert(html).unwrap();

        assert_eq!(expected, markdown);
    }

    #[test]
    fn test_headerless_with_inner_headered() {
        let html = r#"
        <table>
            <tr>
                <td>Outer</td>
                <td>
                    <table>
                        <thead>
                            <tr>
                                <th>Inner H</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td>Inner V</td>
                            </tr>
                        </tbody>
                    </table>
                </td>
            </tr>
        </table>
        "#;

        let expected = indoc!(
            r#"
            Outer

            Inner H

            Inner V
            "#
        )
        .trim();
        let markdown = htmd::HtmlToMarkdown::new().convert(html).unwrap();

        assert_eq!(expected, markdown);
    }

    /// The parser wraps a stray `<tr>` in a `<tbody>`, so a `<table>` holds one
    /// as a direct child only in a tree built elsewhere and handed to the public
    /// `HtmlToMarkdown::tree_to_markdown`. Reading such a row as the body row
    /// the parser would have made of it keeps that entry point from panicking on
    /// a tree it did not parse itself.
    #[test]
    fn a_row_directly_under_a_table_is_read_as_a_body_row() {
        use htmd::{
            Node,
            options::{Options, TranslationMode},
        };
        use html5ever::{LocalName, QualName, ns};
        use markup5ever_rcdom::NodeData;
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

        let markdown = htmd::HtmlToMarkdown::builder()
            .options(Options {
                translation_mode: TranslationMode::Faithful,
                ..Default::default()
            })
            .build()
            .tree_to_markdown(&tree);

        assert_eq!(
            "| A |
| - |
| c |",
            markdown
        );
    }

    #[test]
    fn test_headered_with_inner_headered() {
        let html = r#"
        <table>
            <thead>
                <tr>
                    <th>Outer H1</th>
                    <th>Outer H2</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Outer V1</td>
                    <td>
                        <table>
                            <thead>
                                <tr>
                                    <th>Inner H</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr>
                                    <td>Inner V</td>
                                </tr>
                            </tbody>
                        </table>
                    </td>
                </tr>
            </tbody>
        </table>
        "#;

        let expected = indoc!(
            r#"
            | Outer H1 | Outer H2         |
            | -------- | ---------------- |
            | Outer V1 | Inner H  Inner V |
            "#
        )
        .trim();
        let markdown = htmd::HtmlToMarkdown::new().convert(html).unwrap();

        assert_eq!(expected, markdown);
    }
}
