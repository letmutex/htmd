# HTMD

An HTML to Markdown converter for Rust, inspired by [turndown.js](https://github.com/mixmark-io/turndown).

### Features

- Rich options, same as turndown.js
- Passed [all test cases](https://github.com/mixmark-io/turndown/blob/master/test/index.html) of turndown.js
- Minimum dependencies, it uses only [html5ever](https://github.com/servo/html5ever)
- Fast, it takes less than 200ms to convert a ~1.4MB Wikipedia page on an i5 7th gen CPU

# Usages

Add the dependency

```toml
htmd = { git = "https://github.com/letmutex/htmd" }
```

### Basic

```rust
use htmd::HtmlToMarkdown;

fn main() {
    assert_eq!("# Heading", HtmlToMarkdown::new().convert("<h1>Heading</h1>").unwrap());
}
```

### Skip tags

```rust
use htmd::HtmlToMarkdown;

let converter = HtmlToMarkdown::builder()
    .skip_tags(vec!["script", "style"])
    .build();
assert_eq!("", converter.convert("<script>let x = 0;</script>").unwrap());
```

### Options

```rust
use htmd::{options::Options, HtmlToMarkdown};

let converter = HtmlToMarkdown::builder()
    .options(Options {
        heading_style: htmd::options::HeadingStyle::Setex,
        ..Default::default()
    })
    .build();
assert_eq!("Heading\n=======", converter.convert("<h1>Heading</h1>").unwrap());
```

### Custom tag handlers

```rust
use htmd::HtmlToMarkdown;

let converter = HtmlToMarkdown::builder()
    .add_handler(vec!["svg"], |_: Element| Some("[Svg Image]".to_string()))
    .build();
assert_eq!("[Svg Image]", converter.convert("<svg></svg>").unwrap());
```

# Credits

- [turndown.js](https://github.com/mixmark-io/turndown)
- [html5ever](https://github.com/servo/html5ever)

# License

```
Copyright 2024 letmutex

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
