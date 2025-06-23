use crate::{
    dom::NodeType,
    layout::{BoxProps, BoxType, LayoutBox},
};
use cursive::{
    view::{IntoBoxedView, View, ViewWrapper},
    views::{DummyView, LinearLayout, Panel, TextView},
};

pub type ElementContainer = Box<dyn View>;

pub fn new_element_container() -> ElementContainer {
    (DummyView {}).into_boxed_view()
}

pub fn to_element_container<'a>(layout: LayoutBox<'a>) -> ElementContainer {
    match layout.box_type {
        BoxType::BlockBox(p) | BoxType::InlineBox(p) => match p {
            BoxProps {
                node_type: NodeType::Element(ref element),
                ..
            } => {
                let mut p = Panel::new(LinearLayout::vertical()).title(element.tag_name.clone());
                match element.tag_name.as_str() {
                    _ => {
                        for child in layout.children.into_iter() {
                            p.with_view_mut(|v| v.add_child(to_element_container(child)));
                        }
                    }
                };

                p.into_boxed_view()
            }
            BoxProps {
                node_type: NodeType::Text(ref t),
                ..
            } => {
                // NOTE: This is puppy original behaviour, not a standard one.
                // For your information, CSS Text Module Level 3 specifies how to process whitespaces.
                // See https://www.w3.org/TR/css-text-3/#white-space-processing for further information.
                let text_to_display = t.data.clone();
                let text_to_display = text_to_display.replace("\n", "");
                let text_to_display = text_to_display.trim();
                if text_to_display != "" {
                    TextView::new(text_to_display).into_boxed_view()
                } else {
                    (DummyView {}).into_boxed_view()
                }
            }
        },
        BoxType::AnonymousBox => {
            let mut p = Panel::new(LinearLayout::horizontal());

            for child in layout.children.into_iter() {
                p.with_view_mut(|v| v.add_child(to_element_container(child)));
            }

            p.into_boxed_view()
        }
    }
}
