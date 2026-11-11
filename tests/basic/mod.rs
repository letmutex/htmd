use pretty_assertions::assert_eq;

mod block_syntax_escaping_faithful;
mod elements;
mod elements_faithful;
mod html_escaping;
mod html_escaping_faithful;
mod math_faithful;
mod options_and_handlers;
mod options_and_handlers_faithful;
mod round_trips_faithful;
mod serialization;
mod serialization_faithful;

/// Asserts that `html` comes back as itself. pulldown-cmark ends a block with
/// a line ending, which is the only difference these trips are allowed.
fn assert_round_trips(html: &str) {
    assert_eq!(
        format!("{html}\n"),
        crate::common::round_trip(html),
        "round trip of {html}"
    );
}
