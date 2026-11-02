1.1. Preserved checkbox HTML when Faithful mode serializes a list as inline HTML.

1.2. Rejected checkbox-only items as tasks, preserving their inputs in Faithful mode.

1.3. Added regression tests for HTML preservation, empty descriptions, and rendered task checkbox states.

2.1. Ignored leading HTML comments when recognizing task checkboxes in Pure mode, including paragraph-wrapped tasks, while preserving comments and checkbox HTML in Faithful mode.

2.2. Prevented standalone line breaks and images without a src attribute from qualifying as task descriptions in Pure mode, avoiding task markers that GFM would render as ordinary text.

2.3. Updated has_task_description and its callers to pass the translation mode, replaced duplicate prefix searches, and removed the duplicate task_markers field while retaining its rustdoc summary.

2.4. Added regression tests for leading comments and descriptions dropped in Pure mode. Formatted the changes, restored CRLF line endings, and verified cargo build, cargo clippy -- --deny warnings, and cargo test all pass.
