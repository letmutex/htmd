use std::rc::Rc;

use crate::{HtmlToMarkdown, Node};

/// Input accepted by [`convert`](crate::convert) and [`HtmlToMarkdown::convert`].
pub trait ConvertInput {
    /// Convert `self` to Markdown using `converter`.
    fn convert_with(self, converter: &HtmlToMarkdown) -> std::io::Result<String>;
}

impl ConvertInput for &str {
    #[inline]
    fn convert_with(self, converter: &HtmlToMarkdown) -> std::io::Result<String> {
        let tree = converter.html_to_tree(self)?;
        Ok(converter.tree_to_markdown(&tree))
    }
}

macro_rules! impl_str_convert_input {
    ($($ty:ty),* $(,)?) => {
        $(
            impl ConvertInput for $ty {
                #[inline]
                fn convert_with(self, converter: &HtmlToMarkdown) -> std::io::Result<String> {
                    <&str as ConvertInput>::convert_with(&self, converter)
                }
            }
        )*
    };
}

impl_str_convert_input!(
    &&str,
    &mut str,
    String,
    &String,
    &&String,
    &mut String,
    std::borrow::Cow<'_, str>,
    &std::borrow::Cow<'_, str>,
    Box<str>,
    &Box<str>,
    std::sync::Arc<str>,
    &std::sync::Arc<str>,
    Rc<str>,
    &Rc<str>,
);

impl ConvertInput for &Node {
    #[inline]
    fn convert_with(self, converter: &HtmlToMarkdown) -> std::io::Result<String> {
        Ok(converter.tree_to_markdown(self))
    }
}

impl ConvertInput for &&Node {
    #[inline]
    fn convert_with(self, converter: &HtmlToMarkdown) -> std::io::Result<String> {
        Ok(converter.tree_to_markdown(self))
    }
}

impl ConvertInput for Node {
    #[inline]
    fn convert_with(self, converter: &HtmlToMarkdown) -> std::io::Result<String> {
        Ok(converter.tree_to_markdown(&self))
    }
}

#[cfg(feature = "rcdom")]
impl ConvertInput for &Rc<markup5ever_rcdom::Node> {
    #[inline]
    fn convert_with(self, converter: &HtmlToMarkdown) -> std::io::Result<String> {
        Ok(converter.tree_to_markdown(Node::from_rc_ref(self)))
    }
}

#[cfg(feature = "rcdom")]
impl ConvertInput for Rc<markup5ever_rcdom::Node> {
    #[inline]
    fn convert_with(self, converter: &HtmlToMarkdown) -> std::io::Result<String> {
        Ok(converter.tree_to_markdown(Node::from_rc_ref(&self)))
    }
}
