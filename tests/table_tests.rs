mod common;
use common::convert;

#[cfg(test)]
mod table_tests_1 {
    use super::convert;
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

        let markdown = convert(html).unwrap();
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

        let markdown = convert(html).unwrap();
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

        let expected = r#"
Sample Table
| John | 35 | New York      |
| Jane | 28 | San Francisco |
"#
        .trim();

        let markdown = convert(html).unwrap();
        let result = markdown.trim();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_empty_table() {
        let html = "<table></table>";
        let markdown = convert(html).unwrap();
        let result = markdown.trim();
        assert_eq!("", result);
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

        let markdown = convert(html).unwrap();
        let result = markdown.trim();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_table_block_cells() {
        assert_eq!(
            indoc!(
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
                </table>"#
            ),
            // This has a block (a paragraph) in the table headings.
            convert(indoc!(
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
