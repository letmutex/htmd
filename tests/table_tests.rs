mod common;
use common::convert;

#[cfg(test)]
mod table_tests_1 {
    use super::convert;
    use pretty_assertions::assert_eq;

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
}
