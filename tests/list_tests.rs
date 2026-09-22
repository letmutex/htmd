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
