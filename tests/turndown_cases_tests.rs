use htmd::{
    HtmlToMarkdown,
    options::{
        BrStyle, BulletListMarker, CodeBlockFence, CodeBlockStyle, HeadingStyle, HrStyle,
        LinkReferenceStyle, LinkStyle, Options, TranslationMode,
    },
};
use pretty_assertions::assert_eq;
use scraper::{Html, Selector};
use serde::Deserialize;

struct TestCase {
    pub name: String,
    pub html: String,
    pub md: String,
    pub options: CaseOptions,
}

/// The `data-options` of a case, named as turndown names them.
///
/// Every key is optional and every unknown one is an error: a misspelling used
/// to leave the case running under the defaults, passing while testing something
/// other than what it says. A malformed attribute did worse — html5ever swallows
/// markup into an unterminated one, which silently cost the file a whole case
/// until the parse below started failing on it.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CaseOptions {
    heading_style: Option<String>,
    hr: Option<String>,
    br: Option<String>,
    link_style: Option<String>,
    link_reference_style: Option<String>,
    code_block_style: Option<String>,
    fence: Option<String>,
    bullet_list_marker: Option<String>,
    preformatted_code: Option<bool>,
    translation_mode: Option<String>,
}

#[test]
fn run_cases() {
    let cases = load_test_cases();
    for (index, case) in cases.iter().enumerate() {
        let opt = &case.options;
        let name = case.name.as_str();

        let heading_style = match opt.heading_style.as_deref() {
            Some("atx") => HeadingStyle::Atx,
            Some("setext") | None => HeadingStyle::Setex,
            Some(value) => unsupported(name, "headingStyle", value),
        };

        let hr_style = match opt.hr.as_deref() {
            Some("- - -") => HrStyle::Dashes,
            Some("* * *") | None => HrStyle::Asterisks,
            Some(value) => unsupported(name, "hr", value),
        };

        let link_style = match opt.link_style.as_deref() {
            Some("referenced") => LinkStyle::Referenced,
            Some("inlined") | None => LinkStyle::Inlined,
            Some(value) => unsupported(name, "linkStyle", value),
        };

        let link_reference_style = match opt.link_reference_style.as_deref() {
            Some("collapsed") => LinkReferenceStyle::Collapsed,
            Some("shortcut") => LinkReferenceStyle::Shortcut,
            Some("full") | None => LinkReferenceStyle::Full,
            Some(value) => unsupported(name, "linkReferenceStyle", value),
        };

        // JSON decoding has already turned the attribute's `"\\"` into the one
        // backslash it stands for.
        let br_style = match opt.br.as_deref() {
            Some("\\") => BrStyle::Backslash,
            Some("  ") | None => BrStyle::TwoSpaces,
            Some(value) => unsupported(name, "br", value),
        };

        let code_block_style = match opt.code_block_style.as_deref() {
            Some("fenced") => CodeBlockStyle::Fenced,
            Some("indented") | None => CodeBlockStyle::Indented,
            Some(value) => unsupported(name, "codeBlockStyle", value),
        };

        let code_block_fence = match opt.fence.as_deref() {
            Some("~~~") => CodeBlockFence::Tildes,
            Some("```") | None => CodeBlockFence::Backticks,
            Some(value) => unsupported(name, "fence", value),
        };

        let bullet_list_marker = match opt.bullet_list_marker.as_deref() {
            Some("-") => BulletListMarker::Dash,
            Some("*") | None => BulletListMarker::Asterisk,
            Some(value) => unsupported(name, "bulletListMarker", value),
        };

        let ul_bullet_spacing = 3;
        let ol_number_spacing = 2;

        let preformatted_code = opt.preformatted_code.unwrap_or(false);

        let translation_mode = match opt.translation_mode.as_deref() {
            Some("Pure") => TranslationMode::Pure,
            Some("Faithful") | None => TranslationMode::Faithful,
            Some(value) => unsupported(name, "translationMode", value),
        };

        let converter = HtmlToMarkdown::builder()
            .options(Options {
                heading_style,
                hr_style,
                br_style,
                link_style,
                link_reference_style,
                code_block_style,
                code_block_fence,
                bullet_list_marker,
                ul_bullet_spacing,
                ol_number_spacing,
                preformatted_code,
                translation_mode,
            })
            .build();

        let md = converter.convert(&case.html).unwrap();

        assert_eq!(
            case.md
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&nbsp;", "\u{a0}")
                // For case: list-like text with non-breaking spaces
                .replace("<!-- hard break -->", ""),
            md,
            "Failed on test case '{}' ({}/{})",
            case.name,
            index + 1,
            cases.len()
        );
    }
}

/// A key the runner knows, carrying a value it does not. Like an unknown key,
/// this used to fall back to the default and test the wrong thing quietly.
fn unsupported(case: &str, key: &str, value: &str) -> ! {
    panic!("case {case:?}: unsupported value {value:?} for `{key}`");
}

fn load_test_cases() -> Vec<TestCase> {
    let mut cases = Vec::<TestCase>::new();

    let index_html = std::fs::read_to_string("tests/html/turndown_test_index.html").unwrap();
    let document = Html::parse_document(&index_html);

    let input_selector = Selector::parse("div.input").unwrap();
    let expected_selector = Selector::parse("pre.expected").unwrap();

    for case_element in document.select(&Selector::parse("div.case").unwrap()) {
        let name = case_element.attr("data-name").unwrap();
        let case_input = case_element
            .select(&input_selector)
            .next()
            .unwrap()
            .inner_html();
        let case_expected = case_element
            .select(&expected_selector)
            .next()
            .unwrap()
            .inner_html();
        let options = case_element
            .attr("data-options")
            .map(|attr| {
                serde_json::from_str(attr).unwrap_or_else(|err| {
                    panic!("case {name:?} has unreadable data-options {attr:?}: {err}")
                })
            })
            .unwrap_or_default();
        cases.push(TestCase {
            name: name.to_string(),
            html: case_input,
            md: case_expected,
            options,
        })
    }

    cases
}
