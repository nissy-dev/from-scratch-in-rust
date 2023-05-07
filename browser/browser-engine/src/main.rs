mod css;
mod dom;

fn main() {
    let html = "<html lang='en'><body>Hello, world!</body></html>";
    let dom_tree = dom::parse(html.to_string());
    println!("dom tree\n{:#?}", &dom_tree);
    let css =
        "body { color: #f0f8ff; }\n#container { display: flex; }\n.container { width: 100px; }";
    let style_sheet = css::parse(css.to_string());
    println!("stylesheet\n{:#?}", &style_sheet);
}
