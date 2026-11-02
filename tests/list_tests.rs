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

#[test]
fn task_list_specification() {
    use htmd::{
        HtmlToMarkdown,
        options::{BulletListMarker, TranslationMode},
    };
    let cases = [
        (
            "<ul><li><input type=checkbox> Complete report</li></ul>",
            "- [ ] Complete report",
        ),
        (
            "<ul><li><input type=checkbox checked> Complete report</li></ul>",
            "- [x] Complete report",
        ),
        (
            "<ul><li> <input type=checkbox> Homework<ul><li><input type=checkbox checked> Draft paper</li></ul></li></ul>",
            "- [ ] Homework\n  - [x] Draft paper",
        ),
        (
            "<ul><li><input type=checkbox checked> Write report</li><li>Submit materials</li><li><input type=checkbox> Review feedback</li></ul>",
            "- [x] Write report\n- Submit materials\n- [ ] Review feedback",
        ),
        (
            "<ol><li><input type=checkbox checked> Complete report</li><li><input type=checkbox> Review report</li></ol>",
            "1. [x] Complete report\n2. [ ] Review report",
        ),
        (
            "<ul><li><input type=checkbox checked> Read <strong>chapter 3</strong></li></ul>",
            "- [x] Read **chapter 3**",
        ),
        (
            "<ul><li><p><input type=checkbox> Complete report</p></li></ul>",
            "- [ ] Complete report",
        ),
        (
            "<ul><li>\n\t<input type=CHECKBOX checked=false> Task</li></ul>",
            "- [x] Task",
        ),
        (
            "<ul><li><input type=checkbox> First<p>Second <input type=checkbox></p></li></ul>",
            "- [ ] First\n\n  Second <input type=\"checkbox\">",
        ),
    ];
    for mode in [TranslationMode::Pure, TranslationMode::Faithful] {
        let converter = HtmlToMarkdown::builder()
            .options(Options {
                translation_mode: mode,
                bullet_list_marker: BulletListMarker::Dash,
                ul_bullet_spacing: 1,
                ol_number_spacing: 1,
                ..Default::default()
            })
            .build();
        for (html, expected) in cases {
            let expected = if mode == TranslationMode::Pure && html.contains("Second") {
                "- [ ] First\n\n  Second"
            } else {
                expected
            };
            assert_eq!(
                converter.convert(html).unwrap(),
                expected,
                "{mode:?}: {html}"
            );
        }
    }
}

#[test]
fn ineligible_task_checkboxes() {
    for content in [
        "Complete report <input type=checkbox>",
        "<input type=checkbox> <input type=checkbox> Complete report",
        "<input type=radio> Complete report",
    ] {
        let html = format!("<ul><li>{content}</li></ul>");
        assert_eq!(common::convert_pure(&html).unwrap(), "*   Complete report");
        let faithful = convert_faithful(&html).unwrap();
        assert!(faithful.starts_with("*   "));
        assert!(faithful.contains("<input"));
        assert!(!faithful.contains("[ ]"));
    }
}

#[test]
fn task_attributes_and_spacing() {
    for html in [
        "<ul><li><input type=checkbox disabled> Task</li></ul>",
        "<ul><li><p class=task><input type=checkbox> Task</p></li></ul>",
        "<ul class=tasks><li><input type=checkbox> Task</li></ul>",
        "<ul><li id=task><input type=checkbox> Task</li></ul>",
    ] {
        assert!(convert_faithful(html).unwrap().contains("<input"));
        assert_eq!(common::convert_pure(html).unwrap(), "*   [ ] Task");
    }
    for spacing in [0, 1, 4, 9] {
        let markdown = convert_faithful_options(
            "<ul><li><input type=checkbox> Task</li></ul>",
            Options {
                ul_bullet_spacing: spacing,
                ..Default::default()
            },
        )
        .unwrap();
        let rendered =
            common::render_markdown(&markdown, pulldown_cmark::Options::ENABLE_TASKLISTS);
        assert!(rendered.contains("type=\"checkbox\""), "{markdown}");
    }
}

#[test]
fn task_checkbox_survives_inline_html_serialization() {
    let html = "<h1><ul><li><input type=checkbox> Task</li></ul></h1>";

    assert_eq!(
        convert_faithful(html).unwrap(),
        "# <ul><li><input type=\"checkbox\"> Task</li></ul>"
    );
}

#[test]
fn empty_task_descriptions_preserve_html_in_faithful_mode() {
    for html in [
        "<ul><li><input type=checkbox></li></ul>",
        "<ul><li><p><input type=checkbox> </p></li></ul>",
        "<ul><li><input type=checkbox><span></span></li></ul>",
    ] {
        let markdown = convert_faithful(html).unwrap();

        assert!(markdown.contains("<input type=\"checkbox\">"));
        assert!(!markdown.contains("[ ]"));
    }
}

#[test]
fn task_paragraphs_render_checked_and_unchecked_boxes() {
    use pulldown_cmark::{Event, Options as MarkdownOptions, Parser};

    let html = concat!(
        "<ul>",
        "<li><p><input type=checkbox> Incomplete</p></li>",
        "<li><p><input type=checkbox checked> Complete</p></li>",
        "</ul>",
    );

    let markdown = convert_faithful(html).unwrap();
    let states: Vec<bool> = Parser::new_ext(&markdown, MarkdownOptions::ENABLE_TASKLISTS)
        .filter_map(|event| match event {
            Event::TaskListMarker(checked) => Some(checked),
            _ => None,
        })
        .collect();

    assert_eq!(states, vec![false, true]);
}

#[test]
fn an_image_can_be_a_task_description() {
    use pulldown_cmark::{Event, Options as MarkdownOptions, Parser};

    let html = "<ul><li><input type=checkbox><img src=\"task.png\" alt=\"Task\"></li></ul>";
    let markdown = convert_faithful(html).unwrap();

    let task_count = Parser::new_ext(&markdown, MarkdownOptions::ENABLE_TASKLISTS)
        .filter(|event| matches!(event, Event::TaskListMarker(_)))
        .count();

    assert_eq!(task_count, 1);
}

#[test]
fn leading_comments_do_not_block_pure_task_markers() {
    use pulldown_cmark::{Event, Options as MarkdownOptions, Parser};

    for content in [
        "<!-- comment --><input type=checkbox>Task",
        "<p><!-- comment --><input type=checkbox>Task</p>",
        "<!-- comment --><p><input type=checkbox>Task</p>",
    ] {
        let html = format!("<ul><li>{content}</li></ul>");
        let markdown = common::convert_pure(&html).unwrap();
        let states: Vec<bool> = Parser::new_ext(&markdown, MarkdownOptions::ENABLE_TASKLISTS)
            .filter_map(|event| match event {
                Event::TaskListMarker(checked) => Some(checked),
                _ => None,
            })
            .collect();
        assert_eq!(states, vec![false], "{html}: {markdown}");
        assert!(convert_faithful(&html).unwrap().contains("<input"));
    }
}

#[test]
fn descriptions_dropped_in_pure_mode_do_not_leave_task_markers() {
    for description in ["<br>", "<img>", "<span><img></span>"] {
        let html = format!("<ul><li><input type=checkbox>{description}</li></ul>");
        let markdown = common::convert_pure(&html).unwrap();
        assert!(!markdown.contains("[ ]"), "{html}: {markdown}");
    }
}
