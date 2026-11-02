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

fn task_states(markdown: &str) -> Vec<bool> {
    use pulldown_cmark::{Event, Options as MarkdownOptions, Parser};

    Parser::new_ext(markdown, MarkdownOptions::ENABLE_TASKLISTS)
        .filter_map(|event| match event {
            Event::TaskListMarker(checked) => Some(checked),
            _ => None,
        })
        .collect()
}

#[test]
fn task_whitespace_matches_text_conversion() {
    for wrapped in [false, true] {
        for checked in [false, true] {
            let checked_attr = if checked { " checked" } else { "" };
            for description in ["Task", "\x0c"] {
                let content = format!("\x0c<input type=checkbox{checked_attr}>{description}");
                let content = if wrapped {
                    format!("<p>{content}</p>")
                } else {
                    content
                };
                let html = format!("<ul><li>{content}</li></ul>");

                for markdown in [
                    common::convert_pure(&html).unwrap(),
                    convert_faithful(&html).unwrap(),
                ] {
                    if description == "Task" {
                        assert_eq!(task_states(&markdown), vec![checked], "{html}: {markdown}");
                    } else {
                        assert!(task_states(&markdown).is_empty(), "{html}: {markdown}");
                        assert!(!markdown.contains("[x]"));
                        assert!(!markdown.contains("[ ]"));
                    }
                }
                if description != "Task" {
                    assert!(convert_faithful(&html).unwrap().contains("<input"));
                }
            }
        }
    }
}

#[test]
fn converted_empty_descriptions_do_not_use_later_content() {
    for description in [
        "<span class=\"math math-inline\">% comment</span>",
        "<span class=\"math math-inline\">% comment</span><br>",
    ] {
        for wrapped in [false, true] {
            let content = format!("<input type=checkbox checked>{description}");
            let content = if wrapped {
                format!("<p>{content}</p>")
            } else {
                content
            };
            for suffix in [
                "",
                "<p>Later paragraph</p>",
                "<ul><li><input type=checkbox>Child</li></ul>",
            ] {
                let html = format!("<ul><li>{content}{suffix}</li></ul>");
                let markdown = common::convert_pure(&html).unwrap();
                let expected = if suffix.contains("Child") {
                    vec![false]
                } else {
                    vec![]
                };

                assert_eq!(task_states(&markdown), expected, "{html}: {markdown}");
                assert!(!markdown.contains("[x]"), "{html}: {markdown}");
            }
        }
    }
}

#[test]
fn skipped_task_descriptions_are_converted_once() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use htmd::{Element, HtmlToMarkdown, element_handler::Handlers, options::TranslationMode};

    for mode in [TranslationMode::Pure, TranslationMode::Faithful] {
        for wrapped in [false, true] {
            let calls = Arc::new(AtomicUsize::new(0));
            let recorded_calls = Arc::clone(&calls);
            let converter = HtmlToMarkdown::builder()
                .options(Options {
                    translation_mode: mode,
                    ..Default::default()
                })
                .add_handler(vec!["img"], move |_: &dyn Handlers, _: &Element| {
                    recorded_calls.fetch_add(1, Ordering::Relaxed);
                    None
                })
                .build();
            let content = "<input type=checkbox checked><img src=task.png alt=Task>";
            let content = if wrapped {
                format!("<p>{content}</p>")
            } else {
                content.to_owned()
            };
            let html = format!("<ul><li>{content}</li></ul>");
            let markdown = converter.convert(&html).unwrap();

            assert!(task_states(&markdown).is_empty(), "{mode:?}: {markdown}");
            assert!(!markdown.contains("[x]"), "{mode:?}: {markdown}");
            assert_eq!(calls.load(Ordering::Relaxed), 1, "{mode:?}: {markdown}");
            if mode == TranslationMode::Faithful {
                assert!(markdown.contains("<input type=\"checkbox\" checked=\"\">"));
            }
        }
    }
}

#[test]
fn ordered_tasks_stay_within_the_marker_digit_limit() {
    for start in [999_999_998, 999_999_999, 1_000_000_000] {
        let html = format!(
            "<ol start={start}><li><input type=checkbox checked>First</li><li><input type=checkbox>Second</li></ol>"
        );
        let pure = common::convert_pure(&html).unwrap();
        let faithful = convert_faithful(&html).unwrap();

        assert_eq!(task_states(&pure), vec![true, false], "{pure}");
        if start == 999_999_998 {
            assert_eq!(task_states(&faithful), vec![true, false]);
            assert!(faithful.starts_with("999999998."));
        } else {
            assert!(pure.starts_with("1."));
            assert!(faithful.starts_with(&format!("<ol start=\"{start}\">")));
            assert_eq!(faithful.matches("<input").count(), 2);
            assert!(!faithful.contains("[x]"));
            assert!(!faithful.contains("[ ]"));
        }
    }
}

#[test]
fn custom_task_descriptions_are_checked_after_conversion() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use htmd::{
        Element, HtmlToMarkdown,
        element_handler::{HandlerResult, Handlers},
        options::TranslationMode,
    };

    for mode in [TranslationMode::Pure, TranslationMode::Faithful] {
        for wrapped in [false, true] {
            for html_description in [false, true] {
                let calls = Arc::new(AtomicUsize::new(0));
                let recorded_calls = Arc::clone(&calls);
                let converter = HtmlToMarkdown::builder()
                    .options(Options {
                        translation_mode: mode,
                        ..Default::default()
                    })
                    .add_handler(vec!["span"], move |_: &dyn Handlers, _: &Element| {
                        recorded_calls.fetch_add(1, Ordering::Relaxed);
                        Some(if html_description {
                            HandlerResult {
                                content: "<span>Task</span>".into(),
                                markdown_translated: false,
                            }
                        } else {
                            "Task".into()
                        })
                    })
                    .build();
                let content = "<input type=checkbox checked><span data-label=Task></span>";
                let content = if wrapped {
                    format!("<p>{content}</p>")
                } else {
                    content.to_owned()
                };
                let html = format!("<ul><li>{content}</li></ul>");
                let markdown = converter.convert(&html).unwrap();

                assert_eq!(task_states(&markdown), vec![true], "{mode:?}: {markdown}");
                assert_eq!(calls.load(Ordering::Relaxed), 1, "{mode:?}: {markdown}");
            }
        }
    }
}

#[test]
fn custom_later_paragraphs_do_not_supply_task_descriptions() {
    use htmd::{Element, HtmlToMarkdown, element_handler::Handlers, options::TranslationMode};

    for mode in [TranslationMode::Pure, TranslationMode::Faithful] {
        for replacement in ["\n\nTask", "\r\n \t\r\nTask", "\r\rTask"] {
            let converter = HtmlToMarkdown::builder()
                .options(Options {
                    translation_mode: mode,
                    ul_bullet_spacing: 1,
                    ..Default::default()
                })
                .add_handler(vec!["img"], move |_: &dyn Handlers, _: &Element| {
                    Some(replacement.into())
                })
                .build();
            let markdown = converter
                .convert(
                    "<ul><li><input type=checkbox checked><img src=task.png alt=Task></li></ul>",
                )
                .unwrap();

            assert!(task_states(&markdown).is_empty(), "{mode:?}: {markdown}");
            let html =
                common::render_markdown(&markdown, pulldown_cmark::Options::ENABLE_TASKLISTS);
            let document = scraper::Html::parse_fragment(&html);
            let item = document
                .select(&scraper::Selector::parse("li").unwrap())
                .next()
                .unwrap();
            assert!(item.text().collect::<String>().contains("Task"), "{html}");
            if mode == TranslationMode::Faithful {
                assert!(markdown.contains("<input type=\"checkbox\" checked=\"\">"));
            }
        }
    }
}

#[test]
fn a_backslash_break_cannot_supply_a_task_description() {
    use htmd::{HtmlToMarkdown, options::BrStyle};

    let converter = HtmlToMarkdown::builder()
        .options(Options {
            br_style: BrStyle::Backslash,
            ..Default::default()
        })
        .build();
    for suffix in [
        "",
        "<p>Later paragraph</p>",
        "<ul><li><input type=checkbox>Child</li></ul>",
    ] {
        let html = format!(
            "<ul><li><input type=checkbox checked><span class=\"math math-inline\">%comment</span><br>{suffix}</li></ul>"
        );
        let markdown = converter.convert(&html).unwrap();
        let expected = if suffix.contains("Child") {
            vec![false]
        } else {
            vec![]
        };
        assert_eq!(task_states(&markdown), expected, "{markdown}");
    }
}

#[test]
fn oversized_ordered_task_starts_preserve_faithful_html() {
    for start in [
        "2147483648",
        "18446744073709551616",
        "999999999999999999999999999999999999999999",
        "+2147483648",
    ] {
        let html = format!("<ol start={start}><li><input type=checkbox checked>Task</li></ol>");
        let pure = common::convert_pure(&html).unwrap();
        let faithful = convert_faithful(&html).unwrap();

        assert!(pure.starts_with("1."), "{pure}");
        assert_eq!(task_states(&pure), vec![true], "{pure}");
        assert!(
            faithful.starts_with(&format!("<ol start=\"{start}\">")),
            "{faithful}"
        );
        assert!(faithful.contains("<input type=\"checkbox\" checked=\"\">"));
        assert!(task_states(&faithful).is_empty());
    }
}

#[test]
fn converted_task_descriptions_keep_autolinks_and_ignore_empty_html() {
    use htmd::{
        HtmlToMarkdown,
        options::{LinkStyle, TranslationMode},
    };

    for mode in [TranslationMode::Pure, TranslationMode::Faithful] {
        for destination in ["https://example.com", "mailto:a@example.com"] {
            let html = format!(
                "<ul><li><input type=checkbox checked><a href=\"{destination}\">{destination}</a></li></ul>"
            );
            let options = Options {
                link_style: LinkStyle::InlinedPreferAutolinks,
                ..Default::default()
            };
            let markdown = if mode == TranslationMode::Faithful {
                convert_faithful_options(&html, options).unwrap()
            } else {
                HtmlToMarkdown::builder()
                    .options(options)
                    .build()
                    .convert(&html)
                    .unwrap()
            };
            assert_eq!(task_states(&markdown), vec![true], "{markdown}");
        }
    }

    let html = "<ul><li><input type=checkbox checked><span></span></li></ul>";
    for markdown in [
        common::convert_pure(html).unwrap(),
        convert_faithful(html).unwrap(),
    ] {
        assert!(task_states(&markdown).is_empty(), "{markdown}");
    }

    let html = "<ul><li><input type=checkbox checked><svg><circle r=4></circle></svg></li></ul>";
    assert!(task_states(&common::convert_pure(html).unwrap()).is_empty());
    assert_eq!(task_states(&convert_faithful(html).unwrap()), vec![true]);
}
