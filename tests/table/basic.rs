use indoc::indoc;
use pretty_assertions::assert_eq;

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
    let markdown = htmd::convert("<table>before<tr><th>heading</th></tr>after</table>").unwrap();

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
