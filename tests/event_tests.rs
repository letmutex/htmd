use std::sync::{Arc, Mutex};

use indoc::indoc;
use pretty_assertions::assert_eq;

use htmd::{
    Element, EventSubscription, EventTypes, HtmlToMarkdown,
    element_handler::{ElementHandler, HandlerResult, Handlers},
    options::{Options, TranslationMode},
};

#[test]
fn test_element_events_skip_on_handler_fallback() {
    #[derive(Clone, Default)]
    struct Listener {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl ElementHandler for Listener {
        fn event_subscription(&self, _options: &Options) -> EventSubscription {
            EventSubscription {
                events: EventTypes::ELEMENT_EVENTS,
                tags: &["div"],
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

        fn handle(&self, _handlers: &dyn Handlers, _element: Element) -> Option<HandlerResult> {
            None
        }
    }

    let listener = Listener::default();
    let events = Arc::clone(&listener.events);

    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["watcher"], listener)
        .add_handler(vec!["div"], |_handlers: &dyn Handlers, element: Element| {
            let content = _handlers.walk_children_content(element.node, element.context);
            Some(format!("fallback:{content}").into())
        })
        .add_handler(vec!["div"], |handlers: &dyn Handlers, element: Element| {
            handlers.fallback(element)
        })
        .build();

    let html = indoc!(
        r#"
        <div>Hello</div>
        "#
    );
    let md = converter.convert(html).unwrap();
    assert_eq!("fallback:Hello", md);

    let recorded = events.lock().unwrap().clone();
    assert_eq!(vec!["enter:div", "leave:div:true"], recorded);
}

#[test]
fn test_passive_tag_filtering() {
    #[derive(Clone, Default)]
    struct SelectiveListener {
        entered_tags: Arc<Mutex<Vec<String>>>,
    }

    impl ElementHandler for SelectiveListener {
        fn event_subscription(&self, _options: &Options) -> EventSubscription {
            EventSubscription {
                events: EventTypes::ELEMENT_ENTER,
                tags: &["table", "pre"],
            }
        }

        fn on_element_enter(&self, element: &Element) {
            self.entered_tags
                .lock()
                .unwrap()
                .push(element.tag.to_string());
        }

        fn handle(&self, _handlers: &dyn Handlers, _element: Element) -> Option<HandlerResult> {
            None
        }
    }

    let listener = SelectiveListener::default();
    let entered_tags = Arc::clone(&listener.entered_tags);

    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["listener"], listener)
        .build();

    let html = indoc!(
        r#"
        <h1>Title</h1>
        <p>A paragraph with <span>inline span</span> and <a href="/">link</a>.</p>
        <table>
            <tr><td>cell</td></tr>
        </table>
        <pre><code>code block</code></pre>
        "#
    );

    let _ = converter.convert(html).unwrap();

    let tags = entered_tags.lock().unwrap().clone();
    assert_eq!(vec!["table", "pre"], tags);
}

#[test]
fn test_doc_lifecycle_events_with_nesting() {
    #[derive(Clone, Default)]
    struct DocListener {
        lifecycle: Arc<Mutex<Vec<String>>>,
    }

    impl ElementHandler for DocListener {
        fn event_subscription(&self, _options: &Options) -> EventSubscription {
            EventSubscription {
                events: EventTypes::DOC_EVENTS,
                tags: &[],
            }
        }

        fn on_doc_enter(&self) {
            self.lifecycle.lock().unwrap().push("doc_enter".to_string());
        }

        fn on_doc_leave(&self) {
            self.lifecycle.lock().unwrap().push("doc_leave".to_string());
        }

        fn handle(&self, _handlers: &dyn Handlers, _element: Element) -> Option<HandlerResult> {
            None
        }
    }

    let listener = DocListener::default();
    let lifecycle = Arc::clone(&listener.lifecycle);

    let inner_listener = DocListener {
        lifecycle: Arc::clone(&lifecycle),
    };

    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["watcher"], listener)
        .add_handler(
            vec!["nested"],
            move |_handlers: &dyn Handlers, _element: Element| {
                let inner_conv = HtmlToMarkdown::builder()
                    .add_handler(vec!["watcher"], inner_listener.clone())
                    .build();
                let res = inner_conv.convert("<p>inner</p>").unwrap();
                Some(res.into())
            },
        )
        .build();

    let html = indoc!(
        r#"
        <p>outer start</p>
        <nested></nested>
        <p>outer end</p>
        "#
    );

    let _ = converter.convert(html).unwrap();

    let log = lifecycle.lock().unwrap().clone();
    assert_eq!(
        vec!["doc_enter", "doc_enter", "doc_leave", "doc_leave"],
        log
    );
}

#[test]
fn test_span_events_dispatched_in_pure_mode() {
    #[derive(Clone, Default)]
    struct SpanListener {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl ElementHandler for SpanListener {
        fn event_subscription(&self, _options: &Options) -> EventSubscription {
            EventSubscription {
                events: EventTypes::ELEMENT_EVENTS,
                tags: &["span"],
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

        fn handle(&self, _handlers: &dyn Handlers, _element: Element) -> Option<HandlerResult> {
            None
        }
    }

    let listener = SpanListener::default();
    let events = Arc::clone(&listener.events);

    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["watcher"], listener)
        .build();

    let html = indoc!(
        r#"
        <span>Hello</span>
        "#
    );
    let md = converter.convert(html).unwrap();
    assert_eq!("Hello", md);

    let recorded = events.lock().unwrap().clone();
    assert_eq!(vec!["enter:span", "leave:span:true"], recorded);
}

#[test]
fn test_unhandled_tag_events_dispatched_in_pure_mode() {
    #[derive(Clone, Default)]
    struct CustomTagListener {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl ElementHandler for CustomTagListener {
        fn event_subscription(&self, _options: &Options) -> EventSubscription {
            EventSubscription {
                events: EventTypes::ELEMENT_EVENTS,
                tags: &["custom"],
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

        fn handle(&self, _handlers: &dyn Handlers, _element: Element) -> Option<HandlerResult> {
            None
        }
    }

    let listener = CustomTagListener::default();
    let events = Arc::clone(&listener.events);

    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["watcher"], listener)
        .build();

    let html = indoc!(
        r#"
        <custom>Hello</custom>
        "#
    );
    let md = converter.convert(html).unwrap();
    assert_eq!("Hello", md);

    let recorded = events.lock().unwrap().clone();
    assert_eq!(vec!["enter:custom", "leave:custom:true"], recorded);
}

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

        fn handle(&self, _handlers: &dyn Handlers, _element: Element) -> Option<HandlerResult> {
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

