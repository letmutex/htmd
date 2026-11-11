use std::sync::{Arc, Mutex};

use indoc::indoc;
use pretty_assertions::assert_eq;

use htmd::{
    Element, EventSubscription, EventTypes, HtmlToMarkdown,
    element_handler::{ElementHandler, HandlerResult, Handlers},
    options::{Options, TranslationMode},
};

#[test]
fn test_span_and_unhandled_tag_events_in_faithful_mode() {
    #[derive(Clone, Default)]
    struct MultiTagListener {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl ElementHandler for MultiTagListener {
        fn event_subscription(&self, _options: &Options) -> EventSubscription {
            EventSubscription {
                events: EventTypes::ELEMENT_EVENTS,
                tags: &["span", "custom"],
            }
        }

        fn on_element_enter(&self, element: &Element) {
            self.events
                .lock()
                .unwrap()
                .push(format!("enter:{}", element.tag));
        }

        fn on_element_leave(&self, element: &Element, result: Option<&HandlerResult>) {
            let translated = result.is_some_and(|r| r.markdown_translated);
            self.events
                .lock()
                .unwrap()
                .push(format!("leave:{}:{}", element.tag, translated));
        }

        fn handle(&self, _handlers: &dyn Handlers, _element: &Element) -> Option<HandlerResult> {
            None
        }
    }

    let listener = MultiTagListener::default();
    let events = Arc::clone(&listener.events);

    let options = Options {
        translation_mode: TranslationMode::Faithful,
        ..Default::default()
    };

    let converter = HtmlToMarkdown::builder()
        .options(options)
        .add_handler(vec!["watcher"], listener)
        .build();

    let html = indoc!(
        r#"
        <span>Hello</span>
        <custom>World</custom>
        "#
    );
    let _ = converter.convert(html).unwrap();

    // In Faithful mode, neither `is_passthrough_span` nor `!has_tag_handler` fast path applies,
    // so `ElementHandlers::handle` is invoked for both tags, triggering enter and leave callbacks.
    let recorded = events.lock().unwrap().clone();
    assert_eq!(
        vec![
            "enter:span",
            "leave:span:false",
            "enter:custom",
            "leave:custom:false"
        ],
        recorded
    );
}
