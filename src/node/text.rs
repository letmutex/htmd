#[cfg(feature = "rcdom")]
use std::cell::Ref;
use std::ops::Deref;

pub(super) enum NodeStrInner<'a> {
    Borrowed(&'a str),
    #[cfg(feature = "rcdom")]
    Ref(Ref<'a, str>),
}

/// A borrowed string view backed by a DOM [`Node`](super::Node).
pub struct NodeStr<'a>(pub(super) NodeStrInner<'a>);

impl Deref for NodeStr<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match &self.0 {
            NodeStrInner::Borrowed(s) => s,
            #[cfg(feature = "rcdom")]
            NodeStrInner::Ref(s) => s,
        }
    }
}

impl AsRef<str> for NodeStr<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl std::borrow::Borrow<str> for NodeStr<'_> {
    #[inline]
    fn borrow(&self) -> &str {
        self
    }
}

impl std::hash::Hash for NodeStr<'_> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl PartialOrd for NodeStr<'_> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeStr<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl PartialEq for NodeStr<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Eq for NodeStr<'_> {}

impl PartialEq<str> for NodeStr<'_> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        &**self == other
    }
}

impl PartialEq<&str> for NodeStr<'_> {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        &**self == *other
    }
}

impl PartialEq<String> for NodeStr<'_> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        &**self == other
    }
}

impl PartialEq<NodeStr<'_>> for str {
    #[inline]
    fn eq(&self, other: &NodeStr<'_>) -> bool {
        self == &**other
    }
}

impl PartialEq<NodeStr<'_>> for &str {
    #[inline]
    fn eq(&self, other: &NodeStr<'_>) -> bool {
        *self == &**other
    }
}

impl PartialEq<NodeStr<'_>> for String {
    #[inline]
    fn eq(&self, other: &NodeStr<'_>) -> bool {
        self == &**other
    }
}

impl std::fmt::Display for NodeStr<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

impl std::fmt::Debug for NodeStr<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
