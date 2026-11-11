use pretty_assertions::assert_eq;

use crate::common::convert_faithful;

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
