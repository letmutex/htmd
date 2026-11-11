use html5ever::Attribute;

use crate::Node;

/// The CommonMark context an element is translated in: block vs inline phrasing.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ContextKind {
    /// A block may begin here: the document root and the contents of a
    /// container block.
    Block,
    /// Only inline content may appear here: the contents of a leaf block, and
    /// the contents of a raw HTML inline.
    Inline,
}

/// The context an element or text node is evaluated in.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Context {
    /// The CommonMark syntactic context.
    pub kind: ContextKind,
    /// Whether text in this context is literal
    /// (preserving whitespace without Markdown escaping).
    pub literal: bool,
}

impl Context {
    pub const BLOCK: Self = Self {
        kind: ContextKind::Block,
        literal: false,
    };
    pub const INLINE: Self = Self {
        kind: ContextKind::Inline,
        literal: false,
    };

    /// Whether only inline content may appear here, in which case an element
    /// whose Markdown is a block is written as HTML instead.
    pub fn is_inline(self) -> bool {
        self.kind == ContextKind::Inline
    }

    /// Whether a block may begin here.
    pub fn is_block(self) -> bool {
        self.kind == ContextKind::Block
    }
}

/// The DOM element.
pub struct Element<'a> {
    /// The DOM node of the element.
    pub node: &'a Node,
    /// The tag name.
    pub tag: &'a str,
    pub(crate) attrs: &'a [Attribute],
    /// The [`Context`] this element appears in, as recorded by the walk which
    /// produced it. An element which can only be written as HTML uses this to
    /// pick between an HTML block and a raw HTML inline.
    pub context: Context,
    /// The number of handlers to skip for this element.
    pub(crate) skipped_handlers: usize,
}

impl<'a> Element<'a> {
    /// The value of the first attribute matching `name`, or `None` otherwise.
    #[inline]
    pub fn attr(&self, name: &str) -> Option<&'a str> {
        self.attrs
            .iter()
            .find(|attr| &*attr.name.local == name)
            .map(|attr| &*attr.value)
    }

    /// Iterate lazily over the element's attributes as `(name, value)`.
    #[inline]
    pub fn attrs(
        &self,
    ) -> impl ExactSizeIterator<Item = (&'a str, &'a str)> + DoubleEndedIterator + '_ {
        self.attrs
            .iter()
            .map(|attr| (&*attr.name.local, &*attr.value))
    }
}
