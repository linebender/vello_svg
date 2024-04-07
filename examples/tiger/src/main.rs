use std::path::Path;

use vello::Scene;

fn main() {
    // Reading the svg file into a string
    let assets_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/")
        .canonicalize()
        .unwrap();
    let tiger_path = assets_dir.join("Ghostscript_Tiger.svg");
    let tiger_source =
        std::fs::read_to_string(&tiger_path).expect("Couldn't read svg file {tiger_path:?}");

    // Parsing the source into an usvg tree
    let fontdb = vello_svg::usvg::fontdb::Database::new();
    let svg = vello_svg::usvg::Tree::from_str(
        &tiger_source,
        &vello_svg::usvg::Options::default(),
        &fontdb,
    )
    .expect("Failed to parse svg file {tiger_path:?}");

    // Rendering the tree onto a vello scene
    let mut scene = Scene::new();
    vello_svg::render_tree(&mut scene, &svg);

    // Display the svg
    util::display(svg.size().width() as u32, svg.size().height() as u32, scene)
        .expect("Error while displaying svg");
}
