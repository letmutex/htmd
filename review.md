1.1. Preserved checkbox HTML when Faithful mode serializes a list as inline HTML.

1.2. Rejected checkbox-only items as tasks, preserving their inputs in Faithful mode.

1.3. Added regression tests for HTML preservation, empty descriptions, and rendered task checkbox states.

2.1. Ignored leading HTML comments when recognizing task checkboxes in Pure mode, including paragraph-wrapped tasks, while preserving comments and checkbox HTML in Faithful mode.

2.2. Prevented standalone line breaks and images without a src attribute from qualifying as task descriptions in Pure mode, avoiding task markers that GFM would render as ordinary text.

2.3. Updated has_task_description and its callers to pass the translation mode, replaced duplicate prefix searches, and removed the duplicate task_markers field while retaining its rustdoc summary.

2.4. Added regression tests for leading comments and descriptions dropped in Pure mode. Formatted the changes, restored CRLF line endings, and verified cargo build, cargo clippy -- --deny warnings, and cargo test all pass.

3.1. Reviewed the implementation against the Tasks section of translation_approach.md and identified incomplete recognition of non-text task descriptions and repeated eligibility scans for inputs in the same list item.

3.2. Recognized inline SVG elements as task descriptions in Faithful mode, allowing the checkbox to become a GFM task marker while preserving the SVG as HTML. SVG-only descriptions remain ineligible in Pure mode.

3.3. Passed the current input to task_checkbox and rejected non-leading candidates before scanning the paragraph. Capped recursive checkbox counts at two and stopped the paragraph scan when a second checkbox disqualifies the item.

3.4. Fixed the missing semicolon after try_fold, formatted the code, and restored CRLF line endings. Verified cargo build, cargo clippy -- --deny warnings, and cargo test all pass. Recommended regression tests for SVG descriptions and paragraph-wrapped multiple checkboxes; these tests were not added in this review.

4.1. Kept task-prefix and description recognition consistent with text conversion's ASCII whitespace handling, including form feeds, and formatted the existing whitespace changes.

4.2. Added PendingTask tracking to the existing child walker. Provisional markers are finalized after the initial paragraph is converted, before later blocks or nested lists are processed. Descriptions discarded during conversion no longer leave literal task markers; Faithful mode restores the original checkbox HTML. This preserves the public handler API and converts custom descriptions once.

4.3. Limited ordered Markdown markers to nine digits. Lists whose numbering would cross that limit fall back to HTML in Faithful mode and restart at 1 in Pure mode. Used saturating arithmetic for index calculations and bounded emitted markers.

4.4. Added four integration regression tests covering form-feed prefixes and empty descriptions, comment-only math descriptions with later paragraphs or nested tasks, skipped descriptions and custom-handler call counts, and ordered tasks at the nine-digit boundary. Assertions inspect parsed TaskListMarker states and Faithful checkbox preservation.

4.5. Updated the Tasks section of translation_approach.md, ran cargo fmt, and preserved existing file line endings, including CRLF in the rewritten CRLF files. Verified cargo build, cargo clippy -- --deny warnings, and cargo test all pass.

4.6. Recommend additional tests for SVG-only descriptions in both modes, multiple checkboxes inside a paragraph wrapper, nested ordered-list HTML fallback, and custom handlers that return nonempty descriptions or literal marker-like text.



Final review:

The complete [correction patch](/tmp/htmd-task-corrections.patch) is ready. The code below was validated in a temporary copy; your repository is unchanged.

1. In [li.rs:1](/home/generadium/Projects/School/htmd-raf322/src/element_handler/li.rs:1), update these imports:

```rust
use std::{cell::Cell, ops::Range, rc::Rc};

use html5ever::tokenizer::{BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer};
use markup5ever_rcdom::{Node, NodeData};
```

Replace `has_task_description` at [li.rs:100](/home/generadium/Projects/School/htmd-raf322/src/element_handler/li.rs:100) with this code. It checks converted content, stops at a blank line, and ignores break-only output and empty HTML wrappers.

```rust
fn has_task_description(description: &str, mode: TranslationMode) -> bool {
    let normalized = description.replace("\r\n", "\n").replace('\r', "\n");
    let mut paragraph = String::new();
    for (index, line) in normalized.split_inclusive('\n').enumerate() {
        if index > 0 && line.trim_matches([' ', '\t', '\n']).is_empty() {
            break;
        }
        paragraph.push_str(line);
    }
    let paragraph = paragraph.replace("\\\n", "");
    if !paragraph.contains(['<', '&']) {
        return has_non_whitespace_text(&paragraph);
    }

    let input = BufferQueue::default();
    input.push_back(paragraph.into());
    let tokenizer = Tokenizer::new(
        TaskDescription {
            has_content: Cell::new(false),
            mode,
        },
        Default::default(),
    );
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    tokenizer.sink.has_content.get()
}

struct TaskDescription {
    has_content: Cell<bool>,
    mode: TranslationMode,
}

impl TokenSink for TaskDescription {
    type Handle = ();

    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        let has_content = match token {
            Token::CharacterTokens(text) => has_non_whitespace_text(&text),
            Token::TagToken(tag) => {
                // HTML tokenization also reads Markdown autolinks as tags.
                let is_autolink_or_text = tag
                    .name
                    .bytes()
                    .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'-');
                is_autolink_or_text
                    || (tag.kind == TagKind::StartTag
                        && match tag.name.as_ref() {
                            "br" | "svg" => self.mode == TranslationMode::Faithful,
                            "img" => {
                                self.mode == TranslationMode::Faithful
                                    || tag.attrs.iter().any(|attr| &attr.name.local == "src")
                            }
                            _ => false,
                        })
            }
            _ => false,
        };
        self.has_content.set(self.has_content.get() || has_content);
        TokenSinkResult::Continue
    }
}
```

2. In `PendingTask::finish`, replace its opening condition at [li.rs:148](/home/generadium/Projects/School/htmd-raf322/src/element_handler/li.rs:148) with:

```rust
if has_task_description(
    &output[self.marker_range.end..],
    handlers.options().translation_mode,
) {
    return true;
}
```

Keep the remainder of `finish`.

In `task_checkbox`, delete the following block at [li.rs:211](/home/generadium/Projects/School/htmd-raf322/src/element_handler/li.rs:211). Description eligibility is now determined after conversion.

```rust
let has_description = content
    .iter()
    .take_while(|node| {
        !get_node_tag_name(node).is_some_and(crate::util::node::is_block_element)
    })
    .skip_while(|node| !Rc::ptr_eq(node, checkbox))
    .skip(1)
    .any(|node| has_task_description(node, mode));

if !has_description {
    return None;
}
```

3. In [list.rs:77](/home/generadium/Projects/School/htmd-raf322/src/element_handler/list.rs:77), insert this helper immediately before `get_ol_content`. Oversized positive numbers retain a value above the Markdown limit, allowing the existing fallback to run.

```rust
fn ordered_list_start(value: &str) -> usize {
    let digits = value.strip_prefix('+').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return 1;
    }

    digits
        .bytes()
        .fold(0usize, |number, digit| {
            number
                .saturating_mul(10)
                .saturating_add(usize::from(digit - b'0'))
                .min(MAX_ORDERED_MARKER + 1)
        })
        .max(1)
}
```

Replace the `start_idx` initializer inside `get_ol_content` with:

```rust
let start_idx = element
    .attrs
    .iter()
    .find(|attr| &attr.name.local == "start")
    .map(|attr| ordered_list_start(&attr.value))
    .unwrap_or(1);
```

The proposed changes pass build, Clippy with warnings denied, the full test suite, and five additional regression tests.
Build repair: Restored the PendingTask type and its from_output/finish methods required by dom_walker.rs, removed the incompatible unused finish_task_paragraph helper, and restored validation of converted task descriptions. Verified cargo build, cargo clippy -- --deny warnings, cargo test (including all 26 list tests), and cargo fmt -- --check pass. Preserved CRLF line endings.
