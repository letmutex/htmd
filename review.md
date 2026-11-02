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
