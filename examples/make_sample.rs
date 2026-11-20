#[path = "../tests/draw.rs"]
mod draw;

fn main() {
    let rgb = draw::draw_dashboard(&[0, 4, 2, 1, 3, 7], true, false);
    std::fs::create_dir_all("test/img").expect("create test/img");
    let path = "test/img/sample_dashboard.png";
    image::DynamicImage::ImageRgb8(rgb)
        .save(path)
        .expect("save sample image");
    println!("Wrote {}", path);
}
