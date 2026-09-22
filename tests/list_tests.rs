use htmd::options::Options;
mod common;
use common::{convert_faithful, convert_faithful_options};

#[test]
fn unordered_lists() {
    let html = r#"
        <ul>
            <li>Item 1</li>
            <li>Item 2</li>
            <li>Item 3</li>
        </ul>
        "#;
    assert_eq!(
        "*   Item 1\n*   Item 2\n*   Item 3",
        convert_faithful(html).unwrap()
    )
}

#[test]
fn unordered_lists_custom_bullet_spacing() {
    let html = r#"
        <ul>
            <li>Item 1</li>
            <li>Item 2</li>
            <li>Item 3</li>
        </ul>
        "#;
    let options = Options {
        ul_bullet_spacing: 2,
        ..Default::default()
    };
    let md = convert_faithful_options(html, options).unwrap();
    assert_eq!("*  Item 1\n*  Item 2\n*  Item 3", md)
}

#[test]
fn ordered_lists() {
    let html = r#"
        <ol>
            <li>Item 1</li>
            <li>Item 2</li>
            <li>Item 3</li>
        </ol>
        "#;
    assert_eq!(
        "1.  Item 1\n2.  Item 2\n3.  Item 3",
        convert_faithful(html).unwrap()
    )
}

#[test]
fn ordered_lists_custom_bullet_spacing() {
    let html = r#"
        <ol>
            <li>Item 1</li>
            <li>Item 2</li>
            <li>Item 3</li>
        </ol>
        "#;
    let options = Options {
        ol_number_spacing: 1,
        ..Default::default()
    };
    let md = convert_faithful_options(html, options).unwrap();
    assert_eq!("1. Item 1\n2. Item 2\n3. Item 3", md)
}

#[test]
fn ordered_lists_start_with_zero_or_negative() {
    let html = r#"
        <ol start="0">
            <li>Item 1</li>
            <li>Item 2</li>
            <li>Item 3</li>
        </ol>
        <ol start="-100">
            <li>Item 1</li>
            <li>Item 2</li>
            <li>Item 3</li>
        </ol>
        "#;
    let options = Options {
        ol_number_spacing: 1,
        ..Default::default()
    };
    let md = convert_faithful_options(html, options).unwrap();
    assert_eq!(
        "1. Item 1\n2. Item 2\n3. Item 3\n\n1. Item 1\n2. Item 2\n3. Item 3",
        md
    )
}

#[test]
fn list_spacing_outside_the_commonmark_range() {
    let convert = |html, options| convert_faithful_options(html, options).unwrap();
    let ul = |ul_bullet_spacing| Options {
        ul_bullet_spacing,
        ..Default::default()
    };
    let ol = |ol_number_spacing| Options {
        ol_number_spacing,
        ..Default::default()
    };
    let ul_html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
    let ol_html = "<ol><li>Item 1</li><li>Item 2</li></ol>";

    assert_eq!("* Item 1\n* Item 2", convert(ul_html, ul(0)));
    assert_eq!("*    Item 1\n*    Item 2", convert(ul_html, ul(9)));
    assert_eq!("1. Item 1\n2. Item 2", convert(ol_html, ol(0)));
    assert_eq!("1.    Item 1\n2.    Item 2", convert(ol_html, ol(9)));

    // A nested list is indented to its parent item's content column, so the
    // clamp keeps it a list: past four spaces, CommonMark reads that
    // indentation as a code block instead.
    let nested_ul_html = "<ul><li>Outer<ul><li>Inner</li></ul></li></ul>";
    let nested_ol_html = "<ol><li>Outer<ol><li>Inner</li></ol></li></ol>";

    assert_eq!(
        "*    Outer\n     *    Inner",
        convert(nested_ul_html, ul(9))
    );
    assert_eq!(
        "1.    Outer\n      1.    Inner",
        convert(nested_ol_html, ol(9))
    );
}

#[test]
fn ordered_list_alignment_across_a_digit_boundary() {
    // `start` can put the marker's width change in the middle of the list.
    // Here the padding the narrower markers need stays within the spacing
    // CommonMark allows, so every item's content keeps the same column.
    let html = format!(r#"<ol start="998">{}</ol>"#, "<li>Item</li>".repeat(5));
    let md = convert_faithful(&html).unwrap();
    assert_eq!(
        "998.   Item\n999.   Item\n1000.  Item\n1001.  Item\n1002.  Item",
        md
    );

    let html = format!(r#"<ol start="9">{}</ol>"#, "<li>Item</li>".repeat(2));
    assert_eq!("9.   Item\n10.  Item", convert_faithful(&html).unwrap());
}

#[test]
fn ordered_list_alignment_yields_to_the_spacing_limit() {
    let html = format!("<ol>{}</ol>", "<li>Item</li>".repeat(1000));
    let md = convert_faithful(&html).unwrap();
    let mut lines = md.lines();
    assert_eq!("1.    Item", lines.next().unwrap());
    assert_eq!("1000.  Item", lines.last().unwrap());
}
