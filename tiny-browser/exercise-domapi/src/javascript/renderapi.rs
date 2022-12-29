use cursive::{views::LayerPosition, CbSink};
use std::rc::Rc;

use crate::renderer::Renderer;

pub struct RendererAPI {
    ui_cb_sink: Rc<CbSink>,
}

impl RendererAPI {
    pub fn new(ui_cb_sink: Rc<CbSink>) -> Self {
        Self { ui_cb_sink }
    }

    pub fn rerender(&self) {
        self.ui_cb_sink
            .send(Box::new(move |s: &mut cursive::Cursive| {
                let screen = s.screen_mut();
                let layer: &mut Renderer = screen
                    .get_mut(LayerPosition::FromFront(0))
                    .unwrap()
                    .downcast_mut()
                    .unwrap();
                layer.rerender()
            }))
            .unwrap();
    }
}
