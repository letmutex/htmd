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
fn test_hacker_news_layout_table_does_not_turn_into_giant_padded_rows() {
    let html = include_str!("../../examples/page-to-markdown/html/Hacker News.html");

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
