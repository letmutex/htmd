use std::io;

use ego_tree::NodeRef;
use html5ever::serialize::TraversalScope;
use html5ever::{Attribute, QualName};

use crate::dom::{ChildCursor, DomNode, combining_tag};
use crate::element_handler::element_util::math_delimiter;
use crate::{Node, NodeType};

use super::EgoNodeData;
use super::serialize::serialize_ego_node;

pub(crate) struct EgoChildCursor<'a>(Option<NodeRef<'a, EgoNodeData>>);

impl<'a> ChildCursor<NodeRef<'a, EgoNodeData>> for EgoChildCursor<'a> {
    #[inline]
    fn next(&mut self) -> Option<NodeRef<'a, EgoNodeData>> {
        let current = self.0?;
        self.0 = current.next_sibling();
        Some(current)
    }

    #[inline]
    fn peek(&self) -> Option<NodeRef<'a, EgoNodeData>> {
        self.0
    }
}

impl<'a> DomNode for NodeRef<'a, EgoNodeData> {
    type Child<'c> = NodeRef<'a, EgoNodeData>;
    type Cursor<'c> = EgoChildCursor<'a>;

    #[inline]
    fn node_type(self) -> NodeType {
        match self.value() {
            EgoNodeData::Document | EgoNodeData::DocumentFragment => NodeType::Document,
            EgoNodeData::Doctype { .. } => NodeType::Doctype,
            EgoNodeData::Text(_) => NodeType::Text,
            EgoNodeData::Comment(_) => NodeType::Comment,
            EgoNodeData::Element(_) => NodeType::Element,
            EgoNodeData::ProcessingInstruction { .. } => NodeType::ProcessingInstruction,
        }
    }

    #[inline]
    fn tag_name(&self) -> Option<&str> {
        match self.value() {
            EgoNodeData::Element(elem) => Some(&elem.name.local),
            _ => None,
        }
    }

    #[inline]
    fn as_node(&self) -> Node<'_> {
        Node::from_ego_ref(*self)
    }

    #[inline]
    fn with_text<R>(self, f: impl FnOnce(&str) -> R) -> R {
        let EgoNodeData::Text(contents) = self.value() else {
            unreachable!("expected text node");
        };
        f(contents)
    }

    #[inline]
    fn with_comment<R>(self, f: impl FnOnce(&str) -> R) -> R {
        let EgoNodeData::Comment(contents) = self.value() else {
            unreachable!("expected comment node");
        };
        f(contents)
    }

    #[inline]
    fn with_element<R>(self, f: impl FnOnce(&str, &QualName, &[Attribute]) -> R) -> R {
        let EgoNodeData::Element(elem) = self.value() else {
            unreachable!("expected element node");
        };
        f(&elem.name.local, &elem.name, &elem.attrs)
    }

    #[inline]
    fn with_children_cursor<R>(&self, f: impl for<'c> FnOnce(Self::Cursor<'c>) -> R) -> R {
        f(EgoChildCursor(self.first_child()))
    }

    fn can_combine_with(self, other: Self) -> bool {
        let EgoNodeData::Element(elem1) = self.value() else {
            return false;
        };
        let EgoNodeData::Element(elem2) = other.value() else {
            return false;
        };

        if &elem1.name.local == "a"
            || (elem1.name != elem2.name
                && combining_tag(&elem1.name.local) != combining_tag(&elem2.name.local))
            || elem1.mathml_annotation_xml_integration_point
                != elem2.mathml_annotation_xml_integration_point
            || elem1.template_contents.is_some()
            || elem2.template_contents.is_some()
            || crate::util::node::is_block_element(&elem1.name.local)
        {
            return false;
        }

        let has_single_text = |node: NodeRef<'a, EgoNodeData>| {
            node.first_child().is_some_and(|child| {
                child.next_sibling().is_none() && matches!(child.value(), EgoNodeData::Text(_))
            })
        };
        if !has_single_text(self) || !has_single_text(other) {
            return false;
        }

        !(&elem1.name.local == "span" && math_delimiter(&elem1.attrs).is_some())
            && elem1.attrs == elem2.attrs
    }

    #[inline]
    fn push_single_text(self, out: &mut String) {
        let child = self
            .first_child()
            .expect("compatible elements have one text child");
        let EgoNodeData::Text(contents) = child.value() else {
            unreachable!("compatible elements have one text child");
        };
        out.push_str(contents);
    }

    fn serialize_html(self, traversal_scope: TraversalScope) -> io::Result<String> {
        serialize_ego_node(self, traversal_scope)
    }
}
