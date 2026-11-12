use std::io;
use std::rc::Rc;

use html5ever::serialize::{SerializeOpts, TraversalScope, serialize};
use html5ever::{Attribute, QualName};
use markup5ever_rcdom::{Node as RcDomNode, NodeData, SerializableHandle};

use crate::dom::{ChildCursor, DomNode, combining_tag};
use crate::element_handler::element_util::math_delimiter;
use crate::{Node, NodeType};

pub(crate) struct RcChildCursor<'b>(std::slice::Iter<'b, Rc<RcDomNode>>);

impl<'b> ChildCursor<&'b Rc<RcDomNode>> for RcChildCursor<'b> {
    #[inline]
    fn next(&mut self) -> Option<&'b Rc<RcDomNode>> {
        self.0.next()
    }

    #[inline]
    fn peek(&self) -> Option<&'b Rc<RcDomNode>> {
        self.0.as_slice().first()
    }
}

impl DomNode for &Rc<RcDomNode> {
    type Child<'c> = &'c Rc<RcDomNode>;
    type Cursor<'c> = RcChildCursor<'c>;

    #[inline]
    fn node_type(self) -> NodeType {
        match &self.data {
            NodeData::Document => NodeType::Document,
            NodeData::Doctype { .. } => NodeType::Doctype,
            NodeData::Text { .. } => NodeType::Text,
            NodeData::Comment { .. } => NodeType::Comment,
            NodeData::Element { .. } => NodeType::Element,
            NodeData::ProcessingInstruction { .. } => NodeType::ProcessingInstruction,
        }
    }

    #[inline]
    fn tag_name(&self) -> Option<&str> {
        match &self.data {
            NodeData::Element { name, .. } => Some(&name.local),
            _ => None,
        }
    }

    #[inline]
    fn as_node(&self) -> Node<'_> {
        Node::from_rc_borrowed(self)
    }

    #[inline]
    fn with_text<R>(self, f: impl FnOnce(&str) -> R) -> R {
        let NodeData::Text { contents } = &self.data else {
            unreachable!("expected text node");
        };
        f(&contents.borrow())
    }

    #[inline]
    fn with_comment<R>(self, f: impl FnOnce(&str) -> R) -> R {
        let NodeData::Comment { contents } = &self.data else {
            unreachable!("expected comment node");
        };
        f(contents)
    }

    #[inline]
    fn with_element<R>(self, f: impl FnOnce(&str, &QualName, &[Attribute]) -> R) -> R {
        let NodeData::Element { name, attrs, .. } = &self.data else {
            unreachable!("expected element node");
        };
        f(&name.local, name, &attrs.borrow())
    }

    #[inline]
    fn with_children_cursor<R>(&self, f: impl for<'c> FnOnce(Self::Cursor<'c>) -> R) -> R {
        let children = self.children.borrow();
        f(RcChildCursor(children.iter()))
    }

    fn can_combine_with(self, other: Self) -> bool {
        let NodeData::Element {
            name: name1,
            attrs: attrs1,
            template_contents: template_contents1,
            mathml_annotation_xml_integration_point: mathml_annotation_xml_integration_point1,
        } = &self.data
        else {
            return false;
        };
        let NodeData::Element {
            name: name2,
            attrs: attrs2,
            template_contents: template_contents2,
            mathml_annotation_xml_integration_point: mathml_annotation_xml_integration_point2,
        } = &other.data
        else {
            return false;
        };

        if &name1.local == "a"
            || (name1 != name2 && combining_tag(&name1.local) != combining_tag(&name2.local))
            || mathml_annotation_xml_integration_point1 != mathml_annotation_xml_integration_point2
            || crate::util::node::is_block_element(&name1.local)
        {
            return false;
        }

        let c1 = self.children.borrow();
        let c2 = other.children.borrow();
        if c1.len() != 1
            || c2.len() != 1
            || !matches!(c1[0].data, NodeData::Text { .. })
            || !matches!(c2[0].data, NodeData::Text { .. })
        {
            return false;
        }

        !(&name1.local == "span" && math_delimiter(&attrs1.borrow()).is_some())
            && template_contents1.borrow().is_none()
            && template_contents2.borrow().is_none()
            && attrs1 == attrs2
    }

    #[inline]
    fn push_single_text(self, out: &mut String) {
        let children = self.children.borrow();
        let NodeData::Text { contents } = &children[0].data else {
            unreachable!("compatible elements have one text child");
        };
        out.push_str(&contents.borrow());
    }

    fn serialize_html(self, traversal_scope: TraversalScope) -> io::Result<String> {
        serialize_rc_node(self, traversal_scope)
    }
}

pub(crate) fn serialize_rc_node(
    node: &Rc<RcDomNode>,
    traversal_scope: TraversalScope,
) -> io::Result<String> {
    let scope = match node.data {
        // `markup5ever_rcdom::SerializableHandle` panics on `IncludeNode` for `Document`.
        NodeData::Document => TraversalScope::ChildrenOnly(None),
        _ => traversal_scope,
    };
    let opts = SerializeOpts {
        traversal_scope: scope,
        ..Default::default()
    };
    let mut bytes = Vec::new();
    let handle = SerializableHandle::from(Rc::clone(node));
    serialize(&mut bytes, &handle, opts)?;
    String::from_utf8(bytes).map_err(io::Error::other)
}
