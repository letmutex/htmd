Instructions for AI coding agents
=================================

Commenting guide
----------------

* Use Markdown syntax in doc comments (`///` and `//!`), which rustdoc renders.
  Plain `//` comments are not rendered; write those as prose.

* Use meaningful, descriptive names for variables, classes, functions, etc. Code
  should be as self-documenting as possible.

* Avoid comments when possible. Comments must describe current code, not its
  history. Only add comments which supply what self-documenting code cannot.
  Keep comments as brief as possible.

* Limit comments preceding short functions to one line. Where possible, refer to
  a specification.

* Every public item still gets a rustdoc summary line, even one which only
  restates the signature -- that line is what the generated docs show. The
  criteria below govern the prose which follows it, not the summary.

* Comments must satisfy at least one of these criteria:

  1. Document a connection which cannot easily be determined by inspection --
     for example, the relationship between a web client HTTP request and the
     server endpoint which handles it.
  2. Record behavior discoverable only by running or debugging the code: a
     third-party library quirk, a browser workaround, an ordering constraint.
  3. Capture design choices, requirements, etc. which specify the overall
     purpose of the code at a higher level than the implementation.
  4. Link to an external reference (a manual, specification, etc.) which
     explains a subtle design choice.
* Do not restate the code. Say what the item does, then add what the reader
  cannot derive from the signature:

  ```rust
  // Bad -- the signature already says this.
  /// Set the file's contents to the given string.
  fn set_contents(&mut self, contents: String);

  // Good -- keeps the summary, adds the constraint.
  /// Replace the file's contents. The Client autosaves, so this runs on every
  /// pause in typing; keep it cheap and idempotent.
  fn set_contents(&mut self, contents: String);
  ```

Building and testing
--------------------

`.github/workflows/ci.yaml` gates every pull request on three commands. Run all
three before reporting a change as working:

```
cargo build
cargo clippy -- --deny warnings
cargo test
```

Clippy denies warnings, so code which merely compiles does not pass CI.

Formatting belongs to `cargo fmt` (default settings -- there is no
`rustfmt.toml`); do not hand-align code.

Line endings
------------

The working tree is CRLF (`core.autocrlf=true`, with no `.gitattributes`).
Editing a file with a script which reads text and writes it back converts the
whole file to LF and produces a diff touching every line, so write CRLF back
explicitly -- in Python, `open(path, "w", newline="\r\n")`. `cargo fmt` has the
same effect on the files it rewrites. Git normalizes on commit either way, but
the working tree is left inconsistent.

Repository layout
-----------------

* `src/element_handler/` -- one module per HTML element (`table.rs`, `li.rs`,
  `pre.rs`, ...), each converting that element to Markdown. `mod.rs` defines the
  `ElementHandler` trait and the event types; `element_util.rs` holds helpers
  shared between handlers.
* `src/util/` -- text, escaping, and Unicode helpers used by the handlers.
* `src/lib.rs` and `src/options.rs` -- the public API and the conversion
  options, including `TranslationMode`.
* `tests/` -- integration tests grouped by feature (`table_tests.rs`,
  `link_tests.rs`, ...). Add cases to the file matching the feature, and convert
  through the helpers in `tests/common/mod.rs`, which default to
  `TranslationMode::Faithful`.
* `unsupported_html.md` -- the design record for faithful translation: which
  HTML has no CommonMark equivalent, and why each fallback was chosen. Read it
  before changing how HTML blocks or raw HTML inlines are emitted.
