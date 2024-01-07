use exercise_domapi::{html, renderer::Renderer};
use std::rc::Rc;

const HTML: &str = r#"<body>
    <p>hello</p>
    <p class="inline">world</p>
    <p class="inline">:)</p>
    <div class="none"><p>this should not be shown</p></div>
    <style>
        .none { 
            display: none;
        }
        .inline {
            display: inline;
        }
    </style>

    <div id="result">
        <p>not loaded</p>
    </div>
    <script>
        document.getElementById("result").innerHTML = `\x3cp\x3eloaded\x3c/p\x3e`
    </script>    
</body>"#;

fn main() {
    let mut siv = cursive::default();

    let node = html::parse(HTML);

    let mut renderer = Renderer::new(Rc::new(siv.cb_sink().clone()), node);
    renderer.execute_inline_scripts();

    siv.add_fullscreen_layer(renderer);
    siv.run();
}
