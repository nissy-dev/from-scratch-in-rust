use image::{ImageBuffer, Rgba};

mod css;
mod dom;
mod layout;
mod paint;
mod style;

fn main() {
    let html = "<div class=\"a\">
  <div class=\"b\">
    <div class=\"c\">
      <div class=\"d\">
        <div class=\"e\">
          <div class=\"f\">
            <div class=\"g\">
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>";
    let css = "* { display: block; padding: 12px; }
.a { background: #ff0000; }
.b { background: #ffa500; }
.c { background: #ffff00; }
.d { background: #008000; }
.e { background: #0000ff; }
.f { background: #4b0082; }
.g { background: #800080; }";
    let initial_containing_block = layout::Dimensions {
        content: layout::Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default(),
    };

    let dom_tree = dom::parse(html.to_string());
    let style_sheet = css::parse(css.to_string());
    let style_tree = style::style_tree(&dom_tree, &style_sheet);
    let layout_tree = layout::layout_tree(&style_tree, initial_containing_block);
    let canvas = paint::paint(&layout_tree, initial_containing_block.content);

    // Save an image:
    let (w, h) = (canvas.width as u32, canvas.height as u32);
    let buffer: Vec<Rgba<u8>> = unsafe { std::mem::transmute(canvas.pixels) };
    let img = ImageBuffer::from_fn(w, h, Box::new(|x, y| buffer[(y * w + x) as usize]));
    img.save("output.png").expect("Error saving png image");
}
