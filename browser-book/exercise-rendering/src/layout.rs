use cursive::view::Margins;

use super::{dom::NodeType, style::StyledNode};
use crate::style::{Display, PropertyMap};

#[derive(Debug, PartialEq)]
pub struct LayoutBox<'a> {
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
    pub margin: Margins,
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
    let mut layout = LayoutBox {
        margin: snode.get_margin(),
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
        children: vec![],
    };

    for child in snode.children {
        match child.display() {
            Display::Block => {
                layout.children.push(to_layout_box(child));
            }
            Display::Inline => {
                match layout.children.last() {
                    Some(&LayoutBox {
                        box_type: BoxType::AnonymousBox,
                        ..
                    }) => {}
                    _ => layout.children.push(LayoutBox {
                        box_type: BoxType::AnonymousBox,
                        children: vec![],
                        margin: Margins {
                            top: 0,
                            right: 0,
                            bottom: 0,
                            left: 0,
                        }, // これで良いかは不明
                    }),
                };
                layout
                    .children
                    .last_mut()
                    .unwrap()
                    .children
                    .push(to_layout_box(child));
            }
            Display::None => unreachable!(),
        }
    }

    layout
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
        let inline = [
            (
                "display".to_string(),
                CSSValue::Keyword("inline".to_string()),
            ),
            ("margin".to_string(), CSSValue::Keyword("1".to_string())),
        ];

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
                        margin: Margins {
                            top: 0,
                            right: 0,
                            bottom: 0,
                            left: 0,
                        },
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
                                        margin: Margins {
                                            top: 0,
                                            right: 0,
                                            bottom: 0,
                                            left: 0,
                                        },
                                    },
                                    LayoutBox {
                                        box_type: BoxType::BlockBox(BoxProps {
                                            node_type: &node,
                                            properties: block.iter().cloned().collect(),
                                        }),
                                        children: vec![],
                                        margin: Margins {
                                            top: 0,
                                            right: 0,
                                            bottom: 0,
                                            left: 0,
                                        },
                                    }
                                ],
                                margin: Margins {
                                    top: 1,
                                    right: 1,
                                    bottom: 1,
                                    left: 1,
                                },
                            },
                            LayoutBox {
                                box_type: BoxType::InlineBox(BoxProps {
                                    node_type: &node,
                                    properties: inline.iter().cloned().collect(),
                                }),
                                children: vec![],
                                margin: Margins {
                                    top: 1,
                                    right: 1,
                                    bottom: 1,
                                    left: 1,
                                },
                            }
                        ],
                        margin: Margins {
                            top: 0,
                            right: 0,
                            bottom: 0,
                            left: 0,
                        },
                    },
                    LayoutBox {
                        box_type: BoxType::BlockBox(BoxProps {
                            node_type: &node,
                            properties: block.iter().cloned().collect(),
                        }),
                        children: vec![],
                        margin: Margins {
                            top: 0,
                            right: 0,
                            bottom: 0,
                            left: 0,
                        },
                    }
                ],
                margin: Margins {
                    top: 0,
                    right: 0,
                    bottom: 0,
                    left: 0,
                },
            }
        );
    }
}
