use crate::style::{Display, PropertyMap};
use crate::{dom::NodeType, style::StyledNode};

#[derive(Debug, PartialEq)]
pub struct LayoutBox<'a> {
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum BoxType<'a> {
    BlockBox(BoxProps<'a>),
    InlineBox(BoxProps<'a>),
    AnonymousBox,
}

#[derive(Debug, PartialEq)]
pub struct BoxProps<'a> {
    pub node_type: &'a NodeType,
    pub properties: PropertyMap,
}

pub fn to_layout_box<'a>(snode: StyledNode<'a>) -> LayoutBox<'a> {
    LayoutBox {
        box_type: match snode.display() {
            Display::Block => BoxType::BlockBox(BoxProps {
                node_type: snode.node_type,
                properties: snode.properties,
            }),
            Display::Inline => BoxType::InlineBox(BoxProps {
                node_type: snode.node_type,
                properties: snode.properties,
            }),
            Display::None => unreachable!(),
        },
        children: snode
            .children
            .into_iter()
            .fold(vec![], |mut acc: Vec<LayoutBox>, child| {
                match child.display() {
                    Display::Block => {
                        acc.push(to_layout_box(child));
                        acc
                    }
                    Display::Inline => {
                        match acc.last() {
                            Some(&LayoutBox {
                                box_type: BoxType::AnonymousBox,
                                ..
                            }) => {}
                            _ => acc.push(LayoutBox {
                                box_type: BoxType::AnonymousBox,
                                children: vec![],
                            }),
                        };
                        acc.last_mut().unwrap().children.push(to_layout_box(child));
                        acc
                    }
                    Display::None => unreachable!(),
                }
            }),
    }
}

#[cfg(test)]
mod tests {
    use crate::{css::CSSValue, dom::Element};

    use super::*;

    #[test]
    fn test_to_layout_box() {
        let block = [(
            "display".to_string(),
            CSSValue::Keyword("block".to_string()),
        )];
        let inline = [(
            "display".to_string(),
            CSSValue::Keyword("inline".to_string()),
        )];

        let node = NodeType::Element(Element {
            tag_name: "div".into(),
            attributes: [].iter().cloned().collect(),
        });
        let snode = StyledNode {
            node_type: &node,
            properties: block.iter().cloned().collect(),
            children: vec![
                StyledNode {
                    node_type: &node,
                    properties: block.iter().cloned().collect(),
                    children: vec![],
                },
                StyledNode {
                    node_type: &node,
                    properties: inline.iter().cloned().collect(),
                    children: vec![
                        StyledNode {
                            node_type: &node,
                            properties: block.iter().cloned().collect(),
                            children: vec![],
                        },
                        StyledNode {
                            node_type: &node,
                            properties: block.iter().cloned().collect(),
                            children: vec![],
                        },
                    ],
                },
                StyledNode {
                    node_type: &node,
                    properties: inline.iter().cloned().collect(),
                    children: vec![],
                },
                StyledNode {
                    node_type: &node,
                    properties: block.iter().cloned().collect(),
                    children: vec![],
                },
            ],
        };

        assert_eq!(
            to_layout_box(snode),
            LayoutBox {
                box_type: BoxType::BlockBox(BoxProps {
                    node_type: &node,
                    properties: block.iter().cloned().collect(),
                }),
                children: vec![
                    LayoutBox {
                        box_type: BoxType::BlockBox(BoxProps {
                            node_type: &node,
                            properties: block.iter().cloned().collect(),
                        }),
                        children: vec![],
                    },
                    LayoutBox {
                        box_type: BoxType::AnonymousBox,
                        children: vec![
                            LayoutBox {
                                box_type: BoxType::InlineBox(BoxProps {
                                    node_type: &node,
                                    properties: inline.iter().cloned().collect(),
                                }),
                                children: vec![
                                    LayoutBox {
                                        box_type: BoxType::BlockBox(BoxProps {
                                            node_type: &node,
                                            properties: block.iter().cloned().collect(),
                                        }),
                                        children: vec![],
                                    },
                                    LayoutBox {
                                        box_type: BoxType::BlockBox(BoxProps {
                                            node_type: &node,
                                            properties: block.iter().cloned().collect(),
                                        }),
                                        children: vec![],
                                    }
                                ],
                            },
                            LayoutBox {
                                box_type: BoxType::InlineBox(BoxProps {
                                    node_type: &node,
                                    properties: inline.iter().cloned().collect(),
                                }),
                                children: vec![],
                            }
                        ]
                    },
                    LayoutBox {
                        box_type: BoxType::BlockBox(BoxProps {
                            node_type: &node,
                            properties: block.iter().cloned().collect(),
                        }),
                        children: vec![],
                    }
                ],
            }
        );
    }
}
